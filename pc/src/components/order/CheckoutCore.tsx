'use client';

import { useCallback, useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { Button, Card, Empty, Modal, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, type Paged } from '@/lib/client-api';
import { formatPrice, productImage, skuText } from '@/lib/format';

/**
 * 订单结算页（核心链路）。
 * Vue 基准：pages/order/check.vue（1679 行）+ src/{Address,PayType,CouponInfo,InvoiceInfo}.vue
 * 已实现：收货地址选择、按店铺分组的商品清单、金额汇总、提交订单（二次确认）。
 * 未迁：优惠券/发票/积分余额抵扣/配送方式切换/预售定金/账期支付——这些会走
 * order/check/update 与 updateCoupon，等下一步逐项补，不在这里堆条件。
 * 注意：提交会创建真实待支付订单，所以必须二次确认。
 */
interface AddressRow {
    addressId?: number;
    id?: number;
    consignee?: string;
    mobile?: string;
    address?: string;
    regionName?: string;
    isDefault?: number;
}

interface CartItem {
    cartId?: number;
    productId?: number;
    productName?: string;
    picThumb?: string;
    productPrice?: number | string;
    quantity?: number;
    skuData?: unknown;
    subtotal?: number | string;
}

interface CartGroup {
    shopId?: number;
    shopTitle?: string;
    noShipping?: number;
    carts?: CartItem[];
}

interface CheckTotal {
    productAmount?: number;
    totalCount?: number;
    shippingFee?: number;
    discountAmount?: number;
    couponAmount?: number;
    totalAmount?: number;
    unpaidAmount?: number;
}

interface CheckData {
    cartList?: CartGroup[];
    total?: CheckTotal;
    availablePoints?: number;
}

export default function CheckoutCore({ flowType = 1 }: { flowType?: number }) {
    const { t } = useTranslation();
    const router = useRouter();
    const [addresses, setAddresses] = useState<AddressRow[]>([]);
    const [addressId, setAddressId] = useState(0);
    const [check, setCheck] = useState<CheckData | null>(null);
    const [loading, setLoading] = useState(true);
    const [needLogin, setNeedLogin] = useState(false);
    const [error, setError] = useState('');
    const [pickAddress, setPickAddress] = useState(false);
    const [confirmOpen, setConfirmOpen] = useState(false);
    const [submitting, setSubmitting] = useState(false);

    const loadCheck = useCallback(
        async (nextAddressId: number) => {
            const result = await apiPost<CheckData>('order/check/index', {
                flowType,
                addressId: nextAddressId
            });
            setCheck(result);
        },
        [flowType]
    );

    useEffect(() => {
        const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
        if (!token) {
            setNeedLogin(true);
            setLoading(false);
            return;
        }
        (async () => {
            try {
                const list = await apiGet<Paged<AddressRow>>('user/address/list', { page: 1, size: 99 });
                const records = list.records ?? [];
                setAddresses(records);
                const firstId = records[0]?.addressId ?? records[0]?.id ?? 0;
                setAddressId(firstId);
                await loadCheck(firstId);
            } catch (e) {
                setError((e as Error).message);
            } finally {
                setLoading(false);
            }
        })();
    }, [loadCheck]);

    const chooseAddress = async (row: AddressRow) => {
        const id = row.addressId ?? row.id ?? 0;
        setAddressId(id);
        setPickAddress(false);
        try {
            await loadCheck(id);
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    const submit = async () => {
        setSubmitting(true);
        try {
            const result = await apiPost<{ orderSn?: string; orderId?: number }>('order/check/submit', {
                flowType,
                addressId,
                shippingType: {},
                payTypeId: 0,
                usePoint: 0,
                useBalance: 0,
                useCredit: 0,
                useCouponIds: [],
                selectUserCouponIds: [],
                buyerNote: '',
                invoiceData: {}
            });
            const orderSn = result?.orderSn ?? '';
            Toast.success(t('下单成功'));
            setConfirmOpen(false);
            router.push(`/order/pay${orderSn ? `?id=${orderSn}` : ''}`);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    if (needLogin) return <Empty title={t('请先登录')} description={<a href="/member/login">{t('去登录')}</a>} />;
    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('结算信息加载失败')} description={error} />;
    if (!check) return <Empty title={t('暂无待结算商品')} />;

    const current = addresses.find((item) => (item.addressId ?? item.id) === addressId);
    const total = check.total ?? {};
    const totalAmount = total.totalAmount ?? total.unpaidAmount ?? 0;

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={3}>{t('确认订单')}</Typography.Title>

            <Card>
                {current ? (
                    <div className="pc-address-pick">
                        <div>
                            <div className="pc-addr-name">
                                {current.consignee}
                                {current.isDefault ? <span className="pc-muted">{t('默认')}</span> : null}
                            </div>
                            <div className="pc-muted">{current.mobile}</div>
                            <div className="pc-muted">{[current.regionName, current.address].filter(Boolean).join(' ')}</div>
                        </div>
                        <Button size="small" theme="light" onClick={() => setPickAddress(true)}>
                            {t('切换地址')}
                        </Button>
                    </div>
                ) : (
                    <div className="pc-address-pick">
                        <span className="pc-muted">{t('还没有收货地址')}</span>
                        <a href="/member/address/list">{t('去添加')}</a>
                    </div>
                )}
            </Card>

            {(check.cartList ?? []).map((group, groupIndex) => (
                <Card key={group.shopId ?? groupIndex} title={group.shopTitle || t('自营')}>
                    {(group.carts ?? []).map((item, index) => (
                        <div className="pc-order-item" key={item.cartId ?? index}>
                            <img src={productImage(item)} alt={item.productName ?? ''} />
                            <div className="pc-order-item-main">
                                <div className="pc-ellipsis">{item.productName}</div>
                                <div className="pc-muted">{skuText(item.skuData)}</div>
                            </div>
                            <div className="pc-order-item-price">
                                {formatPrice(item.productPrice)} × {item.quantity ?? 1}
                            </div>
                        </div>
                    ))}
                </Card>
            ))}

            <Card title={t('金额明细')}>
                <div className="pc-amount-row">
                    <span>{t('商品金额')}</span>
                    <span>{formatPrice(total.productAmount)}</span>
                </div>
                <div className="pc-amount-row">
                    <span>{t('运费')}</span>
                    <span>{formatPrice(total.shippingFee)}</span>
                </div>
                <div className="pc-amount-row">
                    <span>{t('优惠')}</span>
                    <span>-{formatPrice(total.discountAmount)}</span>
                </div>
                <div className="pc-amount-row is-total">
                    <span>{t('应付金额')}</span>
                    <span>{formatPrice(totalAmount)}</span>
                </div>
                <div className="pc-muted">
                    {t('优惠券 / 发票 / 积分余额抵扣 / 配送方式切换尚未迁移，本单按默认口径计算。')}
                </div>
            </Card>

            <div className="pc-order-foot">
                <span>{t('共 {count} 件', { count: total.totalCount ?? 0 })}</span>
                <Button theme="solid" type="primary" disabled={!addressId} onClick={() => setConfirmOpen(true)}>
                    {t('提交订单')}
                </Button>
            </div>

            <Modal visible={pickAddress} title={t('选择收货地址')} onCancel={() => setPickAddress(false)} footer={null}>
                <div className="pc-form">
                    {addresses.length === 0 ? <Empty title={t('暂无地址')} /> : null}
                    {addresses.map((row, index) => (
                        <button
                            key={row.addressId ?? row.id ?? index}
                            type="button"
                            className={`pc-address-option${(row.addressId ?? row.id) === addressId ? ' is-active' : ''}`}
                            onClick={() => void chooseAddress(row)}
                        >
                            <div className="pc-addr-name">{row.consignee} <span className="pc-muted">{row.mobile}</span></div>
                            <div className="pc-muted">{[row.regionName, row.address].filter(Boolean).join(' ')}</div>
                        </button>
                    ))}
                </div>
            </Modal>

            <Modal
                visible={confirmOpen}
                title={t('确认提交订单')}
                onOk={() => void submit()}
                onCancel={() => setConfirmOpen(false)}
                okText={t('确认下单')}
                cancelText={t('取消')}
                confirmLoading={submitting}
            >
                {t('将创建一笔真实待支付订单，应付 {amount}。', { amount: formatPrice(totalAmount) })}
            </Modal>
        </div>
    );
}
