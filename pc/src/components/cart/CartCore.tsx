'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import { Button, Card, Empty, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice, productImage, skuText } from '@/lib/format';

/**
 * 购物车。
 * Vue 基准：pages/cart/index.vue（2249 行）。
 * 已实现：分组清单、单选/全选、改数量、删除、清空、合计、去结算。
 * 未迁：换购/加价购、赠品、限购提示、失效商品分组、店铺满减凑单。
 */
interface CartItem {
    cartId: number;
    productId?: number;
    productName?: string;
    picThumb?: string;
    price?: string | number;
    quantity?: number;
    skuData?: unknown;
    stock?: number;
    isChecked?: boolean;
    isDisabled?: boolean;
    subtotal?: number;
}

interface CartShop {
    shopId?: number;
    shopTitle?: string;
    isChecked?: boolean;
    carts?: CartItem[];
}

interface CartResult {
    cartList?: CartShop[];
}

export default function CartCore() {
    const { t } = useTranslation();
    const [groups, setGroups] = useState<CartShop[]>([]);
    const [loading, setLoading] = useState(true);
    const [needLogin, setNeedLogin] = useState(false);
    const [error, setError] = useState('');
    const [busy, setBusy] = useState(false);

    const load = useCallback(async () => {
        const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
        if (!token) {
            setNeedLogin(true);
            setLoading(false);
            return;
        }
        setLoading(true);
        try {
            const result = await apiGet<CartResult>('cart/cart/list');
            setGroups(result.cartList ?? []);
            setError('');
        } catch (e) {
            setGroups([]);
            setError((e as Error).message);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        void load();
    }, [load]);

    const allItems = useMemo(() => groups.flatMap((group) => group.carts ?? []), [groups]);
    const checkedItems = allItems.filter((item) => item.isChecked);
    const totalAmount = checkedItems.reduce((sum, item) => sum + Number(item.price ?? 0) * Number(item.quantity ?? 1), 0);

    const toggleItem = async (item: CartItem) => {
        setBusy(true);
        try {
            await apiPost('cart/cart/updateCheck', { cartId: item.cartId, isChecked: !item.isChecked });
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setBusy(false);
        }
    };

    const toggleAll = async () => {
        setBusy(true);
        try {
            const target = !allItems.every((item) => item.isChecked);
            await Promise.all(allItems.map((item) => apiPost('cart/cart/updateCheck', { cartId: item.cartId, isChecked: target })));
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setBusy(false);
        }
    };

    const changeQuantity = async (item: CartItem, next: number) => {
        if (next < 1) return;
        setBusy(true);
        try {
            await apiPost('cart/cart/updateItem', { cartId: item.cartId, data: { quantity: next } });
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setBusy(false);
        }
    };

    const remove = async (item: CartItem) => {
        setBusy(true);
        try {
            await apiPost('cart/cart/removeItem', { cartIds: [item.cartId] });
            Toast.success(t('已移除'));
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setBusy(false);
        }
    };

    const clear = async () => {
        setBusy(true);
        try {
            await apiPost('cart/cart/clear');
            Toast.success(t('已清空'));
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setBusy(false);
        }
    };

    if (needLogin) return <Empty title={t('请先登录')} description={<a href="/member/login">{t('去登录')}</a>} />;
    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('购物车加载失败')} description={error} />;
    if (groups.length === 0) return <Empty title={t('购物车是空的')} description={<Link href="/list">{t('去逛逛')}</Link>} />;

    return (
        <div className="pc-cart">
            <div className="pc-panel-head">
                <Typography.Title heading={3}>{t('购物车')}</Typography.Title>
                <div className="pc-order-actions">
                    <Button size="small" theme="light" disabled={busy} onClick={() => void toggleAll()}>
                        {allItems.every((item) => item.isChecked) ? t('取消全选') : t('全选')}
                    </Button>
                    <Button size="small" theme="light" disabled={busy} onClick={() => void clear()}>
                        {t('清空')}
                    </Button>
                </div>
            </div>

            {groups.map((group, groupIndex) => (
                <Card key={group.shopId ?? groupIndex} title={group.shopTitle || t('自营')}>
                    {(group.carts ?? []).map((item) => (
                        <div className={`pc-cart-row${item.isDisabled ? ' is-disabled' : ''}`} key={item.cartId}>
                            <input
                                type="checkbox"
                                checked={!!item.isChecked}
                                disabled={busy || item.isDisabled}
                                onChange={() => void toggleItem(item)}
                            />
                            <img src={productImage(item)} alt={item.productName ?? ''} />
                            <div className="pc-cart-main">
                                <div className="pc-ellipsis">{item.productName}</div>
                                <div className="pc-muted">{skuText(item.skuData)}</div>
                                {item.isDisabled ? <div className="pc-muted">{t('已失效')}</div> : null}
                            </div>
                            <div className="pc-cart-price">{formatPrice(item.price)}</div>
                            <div className="pc-cart-qty">
                                <button type="button" disabled={busy} onClick={() => void changeQuantity(item, Number(item.quantity ?? 1) - 1)}>
                                    -
                                </button>
                                <span>{item.quantity ?? 1}</span>
                                <button type="button" disabled={busy} onClick={() => void changeQuantity(item, Number(item.quantity ?? 1) + 1)}>
                                    +
                                </button>
                            </div>
                            <div className="pc-cart-subtotal">{formatPrice(Number(item.price ?? 0) * Number(item.quantity ?? 1))}</div>
                            <button type="button" className="pc-cart-remove" disabled={busy} onClick={() => void remove(item)}>
                                {t('删除')}
                            </button>
                        </div>
                    ))}
                </Card>
            ))}

            <div className="pc-cart-bar">
                <span>
                    {t('已选 {count} 件', { count: checkedItems.length })} · {t('合计')} <strong>{formatPrice(totalAmount)}</strong>
                </span>
                <Link href="/order/check">
                    <Button theme="solid" type="primary" disabled={checkedItems.length === 0}>
                        {t('去结算')}
                    </Button>
                </Link>
            </div>
        </div>
    );
}
