'use client';

import { useEffect, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { Button, Card, Empty, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice, skuText } from '@/lib/format';
import { CountdownBoxes, useCountdown } from '@/components/promotion/cards';

/**
 * 商品详情。
 * Vue 基准：pages/item/src/Product.vue + components/product/**。
 *
 * **真实接口口径（实测 2026-11-13）**：
 * - `product/product/detail?sn=&grouponPromotionRecordId=` → { item, skuList, picList, serviceList,
 *   grouponDetail, seckillDetail, activityInfo… }
 * - `cart/cart/addToCart` 的参数是 **`{ id, number, skuId }`**（id=商品ID、number=数量），
 *   不是 `{ productId, quantity }`；返回 `data` = cartId。
 * - 购物车条目会带 `activityInfo[{ promotionId, promotionName, type, typeText }]`，
 *   **type=7 是拼团**，结算页据此识别拼团单。
 *
 * 已实现：主图、名称、价格、SKU、数量、加入购物车、立即购买、活动角标、拼团信息。
 * 未迁：图文详情、评价、咨询、领券、秒杀下单按钮（秒杀只展示价）。
 */
interface SkuRow {
    skuId?: number;
    id?: number;
    skuName?: string;
    skuData?: unknown;
    productPrice?: number | string;
    skuPrice?: number | string;
    price?: number | string;
    stock?: number;
    skuStock?: number;
}

interface ActivityInfo {
    promotionId?: number;
    promotionName?: string;
    type?: number;
    typeText?: string;
}

interface ProductItem {
    productId?: number;
    productName?: string;
    productSn?: string;
    productPrice?: number | string;
    picThumb?: string;
    stock?: number;
    productStock?: number;
    unit?: string;
    activityInfo?: ActivityInfo[];
}

interface GrouponDetail {
    productId?: number;
    minGrouponPrice?: string | number;
    minGroupNum?: number;
    joinNumCount?: number;
    endTime?: string | number;
    grouponPromotionItemId?: number;
    canJoin?: boolean;
    restrictTip?: string;
}

interface DetailResult {
    item?: ProductItem;
    skuList?: SkuRow[];
    picList?: Array<{ picUrl?: string; picThumb?: string }>;
    serviceList?: Array<{ name?: string; content?: string }>;
    grouponDetail?: GrouponDetail | null;
    seckillDetail?: { seckillPrice?: string | number; seckillStock?: number } | null;
}

export default function ProductDetail({ idOrSn }: { idOrSn: string }) {
    const { t } = useTranslation();
    const router = useRouter();
    const searchParams = useSearchParams();
    // 被邀请参团进入时带 recordId：透传给详情接口，按钮文案变「参与拼团」（Vue 同口径）
    const grouponRecordId = Number(searchParams?.get('grouponPromotionRecordId')) || 0;
    const invited = (searchParams?.get('type') ?? '') === 'invited';
    const [detail, setDetail] = useState<DetailResult | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [skuId, setSkuId] = useState<number>(0);
    const [quantity, setQuantity] = useState(1);
    const [pending, setPending] = useState<'' | 'cart' | 'buy'>('');

    const groupon = detail?.grouponDetail ?? null;
    const grouponCd = useCountdown(groupon?.endTime ?? 0);

    useEffect(() => {
        if (!idOrSn) {
            setError(t('缺少商品参数'));
            setLoading(false);
            return;
        }
        const numeric = Number(idOrSn);
        const params =
            Number.isFinite(numeric) && numeric > 0
                ? { id: numeric, grouponPromotionRecordId: grouponRecordId || undefined }
                : { sn: idOrSn, grouponPromotionRecordId: grouponRecordId || undefined };
        apiGet<DetailResult>('product/product/detail', params)
            .then((result) => {
                setDetail(result);
                const firstSku = result.skuList?.[0];
                setSkuId(firstSku?.skuId ?? firstSku?.id ?? 0);
            })
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [idOrSn, t, grouponRecordId]);

    const addToCart = async (thenCheckout = false) => {
        const productId = detail?.item?.productId;
        if (!productId) return Toast.warning(t('商品信息不完整'));
        setPending(thenCheckout ? 'buy' : 'cart');
        try {
            // 参数名必须是 id / number（Vue 的 addToCart 同样如此）
            await apiPost('cart/cart/addToCart', { id: productId, number: quantity, skuId: skuId || undefined });
            if (thenCheckout) {
                router.push('/order/check');
                return;
            }
            Toast.success(t('已加入购物车'));
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setPending('');
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('商品加载失败')} description={error} />;
    if (!detail?.item) return <Empty title={t('商品不存在')} />;

    const item = detail.item;
    const skus = detail.skuList ?? [];
    const activeSku = skus.find((sku) => (sku.skuId ?? sku.id) === skuId);
    const price = activeSku?.productPrice ?? activeSku?.skuPrice ?? activeSku?.price ?? item.productPrice;
    const stock = activeSku?.stock ?? activeSku?.skuStock ?? item.stock ?? item.productStock;
    const cover = detail.picList?.[0]?.picUrl ?? detail.picList?.[0]?.picThumb ?? item.picThumb;
    const activities = (item.activityInfo ?? []).filter((row) => row.typeText || row.promotionName);
    const soldOut = Number(stock ?? 0) <= 0;

    return (
        <div className="pc-product-detail">
            <Card>
                <div className="pc-pd-top">
                    <div className="pc-pd-gallery">
                        <img src={cover || '/assets/avatar/default.gif'} alt={item.productName ?? ''} />
                    </div>
                    <div className="pc-pd-info">
                        <Typography.Title heading={3}>{item.productName}</Typography.Title>

                        {groupon ? (
                            <div className="pc-pd-groupon">
                                <div>
                                    <span className="pc-promo-price">{formatPrice(groupon.minGrouponPrice)}</span>
                                    <span className="pc-promo-origin">{formatPrice(price)}</span>
                                    {groupon.minGroupNum ? (
                                        <Tag size="small" color="red">
                                            {groupon.minGroupNum}
                                            {t('人团')}
                                        </Tag>
                                    ) : null}
                                </div>
                                <div className="pc-muted">
                                    {t('已参团')} {groupon.joinNumCount ?? 0}
                                    {groupon.endTime ? (
                                        <>
                                            {' · '}
                                            {t('距结束')} <CountdownBoxes cd={grouponCd} endedText={t('已结束')} />
                                        </>
                                    ) : null}
                                </div>
                                {groupon.canJoin === false ? (
                                    <div className="pc-muted">{groupon.restrictTip || t('仅限指定会员参与')}</div>
                                ) : null}
                            </div>
                        ) : (
                            <div className="pc-pd-price">{formatPrice(price)}</div>
                        )}

                        <div className="pc-muted">
                            {t('货号')} {item.productSn ?? '-'} · {t('库存')} {stock ?? '-'}
                        </div>

                        {activities.length > 0 ? (
                            <div className="pc-pd-activities">
                                {activities.map((row, index) => (
                                    <Tag key={row.promotionId ?? index} size="small" color="blue">
                                        {row.typeText ? `${row.typeText}：` : ''}
                                        {row.promotionName ?? ''}
                                    </Tag>
                                ))}
                            </div>
                        ) : null}

                        {skus.length > 0 ? (
                            <div className="pc-pd-skus">
                                {skus.map((sku) => {
                                    const key = sku.skuId ?? sku.id ?? 0;
                                    return (
                                        <button
                                            key={key}
                                            type="button"
                                            className={`pc-pay-channel${key === skuId ? ' is-active' : ''}`}
                                            onClick={() => setSkuId(key)}
                                        >
                                            {sku.skuName ?? skuText(sku.skuData) ?? key}
                                        </button>
                                    );
                                })}
                            </div>
                        ) : null}

                        <div className="pc-cart-qty">
                            <button type="button" onClick={() => setQuantity(Math.max(1, quantity - 1))}>
                                -
                            </button>
                            <span>{quantity}</span>
                            <button type="button" onClick={() => setQuantity(quantity + 1)}>
                                +
                            </button>
                        </div>

                        <div className="pc-order-actions">
                            <Button
                                theme="solid"
                                type="primary"
                                loading={pending === 'cart'}
                                disabled={soldOut}
                                onClick={() => void addToCart(false)}
                            >
                                {soldOut ? t('已售罄') : t('加入购物车')}
                            </Button>
                            <Button
                                theme="light"
                                loading={pending === 'buy'}
                                disabled={soldOut}
                                onClick={() => void addToCart(true)}
                            >
                                {groupon ? (invited ? t('参与拼团') : t('我要开团')) : t('立即购买')}
                            </Button>
                            <a href="/cart">
                                <Button theme="borderless">{t('去购物车')}</Button>
                            </a>
                        </div>

                        {(detail.serviceList ?? []).length > 0 ? (
                            <div className="pc-muted">
                                {(detail.serviceList ?? []).map((service, index) => (
                                    <span key={index}>{service.name} </span>
                                ))}
                            </div>
                        ) : null}
                    </div>
                </div>
            </Card>

            <div className="pc-note">
                {t('图文详情、评价、咨询、领券、秒杀下单按钮尚未迁移；主图 + SKU + 加购/立即购买已可用。')}
                {groupon ? ` ${t('拼团单提交后可在「我的拼团」查看成团进度。')}` : ''}
            </div>
        </div>
    );
}
