'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useRouter } from 'next/navigation';
import { Button, Card, Empty, Input, Modal, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, type Paged } from '@/lib/client-api';
import { formatPrice, productImage, skuText } from '@/lib/format';
import InvoiceDialog, { EMPTY_INVOICE, type InvoiceValue } from '@/components/invoice/InvoiceDialog';

/**
 * 订单结算页。
 * Vue 基准：pages/order/check.vue（1679 行）+ src/{Address,PayType,CouponInfo,InvoiceInfo}.vue
 * 已实现：地址选择、配送方式、商品清单、优惠券、发票（普通/专用/个人/企业完整表单）、
 *         积分/余额抵扣、金额重算、提交。
 * 拼团单：购物车条目 activityInfo 里含 type=7 时提示拼团，提交后按后端 returnType 分流
 * （returnType=2 不走收银台，带 grouponPromotionRecordId 时跳「我的拼团」详情）。
 * 未迁：预售定金、账期支付、支付方式选择（提交时 payTypeId=0）、定金预售步骤条。
 * 所有计价变更统一走 order/check/update；优惠券走 order/check/updateCoupon。
 * 提交会创建真实待支付订单，因此必须二次确认。
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

interface CartActivity {
    promotionId?: number;
    promotionName?: string;
    /** 2=优惠券 7=拼团（结算页据此识别拼团单） */
    type?: number;
    typeText?: string;
}

interface CartItem {
    cartId?: number;
    productId?: number;
    productName?: string;
    picThumb?: string;
    productPrice?: number | string;
    quantity?: number;
    skuData?: unknown;
    activityInfo?: CartActivity[];
}

interface CartGroup {
    shopId?: number;
    shopTitle?: string;
    carts?: CartItem[];
}

interface CheckTotal {
    productAmount?: number;
    totalCount?: number;
    shippingFee?: number;
    discountAmount?: number;
    couponAmount?: number;
    pointsAmount?: number;
    balance?: number;
    totalAmount?: number;
    unpaidAmount?: number;
}

interface CouponItem {
    id?: number;
    userCouponId?: number;
    couponName?: string;
    couponMoney?: string | number;
    couponDiscount?: string | number;
    minOrderAmount?: string | number;
    shopId?: number;
}

interface CheckData {
    cartList?: CartGroup[];
    total?: CheckTotal;
    availablePoints?: number;
    couponList?: { enableCoupons?: CouponItem[] };
    useCouponIds?: number[];
    selectUserCouponIds?: number[];
}

interface ShippingOption {
    shippingTypeId: number;
    shippingTypeName: string;
    shopId?: number;
}

type ShippingMap = Record<string, ShippingOption[]>;
type ShippingState = Record<string, { typeId: number; shopId: number; typeName: string }>;

export default function CheckoutCore({ flowType = 1 }: { flowType?: number }) {
    const { t } = useTranslation();
    const router = useRouter();
    const [addresses, setAddresses] = useState<AddressRow[]>([]);
    const [addressId, setAddressId] = useState(0);
    const [check, setCheck] = useState<CheckData | null>(null);
    const [shippingMap, setShippingMap] = useState<ShippingMap>({});
    const [shipping, setShipping] = useState<ShippingState>({});
    const [usePoint, setUsePoint] = useState(0);
    const [useBalance, setUseBalance] = useState(0);
    const [couponIds, setCouponIds] = useState<number[]>([]);
    const [userCouponIds, setUserCouponIds] = useState<number[]>([]);
    const [invoice, setInvoice] = useState<InvoiceValue>(EMPTY_INVOICE);
    const [invoiceAdded, setInvoiceAdded] = useState(false);
    const [loading, setLoading] = useState(true);
    const [needLogin, setNeedLogin] = useState(false);
    const [recalculating, setRecalculating] = useState(false);
    const [error, setError] = useState('');
    const [pickAddress, setPickAddress] = useState(false);
    const [pickCoupon, setPickCoupon] = useState(false);
    const [pickInvoice, setPickInvoice] = useState(false);
    const [confirmOpen, setConfirmOpen] = useState(false);
    const [submitting, setSubmitting] = useState(false);

    const buildPayload = useCallback(
        (patch: Record<string, unknown> = {}) => ({
            flowType,
            addressId,
            shippingType: shipping,
            payTypeId: 0,
            usePoint,
            useBalance,
            useCredit: 0,
            useCouponIds: couponIds,
            selectUserCouponIds: userCouponIds,
            buyerNote: '',
            invoiceData: invoice,
            ...patch
        }),
        [flowType, addressId, shipping, usePoint, useBalance, couponIds, userCouponIds, invoice]
    );

    const loadCheck = useCallback(
        async (nextAddressId: number) => {
            const result = await apiPost<CheckData>('order/check/index', { flowType, addressId: nextAddressId });
            setCheck(result);
            setCouponIds(result.useCouponIds ?? []);
            setUserCouponIds(result.selectUserCouponIds ?? []);
        },
        [flowType]
    );

    const recalc = useCallback(
        async (patch: Record<string, unknown>) => {
            setRecalculating(true);
            try {
                const result = await apiPost<CheckData>('order/check/update', buildPayload(patch));
                setCheck(result);
            } catch (e) {
                Toast.error((e as Error).message);
            } finally {
                setRecalculating(false);
            }
        },
        [buildPayload]
    );

    useEffect(() => {
        const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
        if (!token) {
            setNeedLogin(true);
            setLoading(false);
            return;
        }
        // 发票开关：invoiceAdded === 1 才允许开增值税专用发票
        apiGet<{ invoiceAdded?: number; canInvoice?: number }>('common/config/initConfigSettings')
            .then((config) => setInvoiceAdded(Number(config?.invoiceAdded) === 1))
            .catch(() => setInvoiceAdded(false));
        (async () => {
            try {
                const [list, shippingResult] = await Promise.all([
                    apiGet<Paged<AddressRow>>('user/address/list', { page: 1, size: 99 }),
                    apiGet<ShippingMap>('order/check/getStoreShippingType', { flowType }).catch((): ShippingMap => ({}))
                ]);
                const records = list.records ?? [];
                setAddresses(records);
                const firstId = records[0]?.addressId ?? records[0]?.id ?? 0;
                setAddressId(firstId);

                const map = shippingResult ?? {};
                setShippingMap(map);
                const initial: ShippingState = {};
                Object.keys(map).forEach((shopId) => {
                    const first = map[shopId]?.[0];
                    if (first) {
                        initial[shopId] = { typeId: first.shippingTypeId, shopId: Number(shopId), typeName: first.shippingTypeName };
                    }
                });
                setShipping(initial);

                await loadCheck(firstId);
            } catch (e) {
                setError((e as Error).message);
            } finally {
                setLoading(false);
            }
        })();
    }, [flowType, loadCheck]);

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

    const chooseShipping = async (shopId: number, option: ShippingOption) => {
        const next: ShippingState = {
            ...shipping,
            [shopId]: { typeId: option.shippingTypeId, shopId, typeName: option.shippingTypeName }
        };
        setShipping(next);
        await recalc({ shippingType: next, addressId });
    };

    const applyCoupon = async (ids: number[], userIds: number[]) => {
        setCouponIds(ids);
        setUserCouponIds(userIds);
        setRecalculating(true);
        try {
            const result = await apiPost<CheckData>('order/check/updateCoupon', {
                ...buildPayload(),
                useCouponIds: ids,
                selectUserCouponIds: userIds,
                useDefaultCouponIds: 0
            });
            setCheck(result);
            setCouponIds(result.useCouponIds ?? ids);
            setUserCouponIds(result.selectUserCouponIds ?? userIds);
            setPickCoupon(false);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setRecalculating(false);
        }
    };

    const toggleCoupon = (item: CouponItem) => {
        const userCouponId = Number(item.userCouponId ?? 0);
        const couponId = Number(item.id ?? 0);
        const exists = userCouponIds.includes(userCouponId);
        let nextUsers = userCouponIds.filter((id) => id !== userCouponId);
        let nextIds = couponIds.filter((id) => id !== couponId);
        if (!exists) {
            // 同店铺只能选一张：移除该店铺已选券
            const sameShop = (check?.couponList?.enableCoupons ?? []).filter(
                (coupon) => coupon.shopId === item.shopId && userCouponIds.includes(Number(coupon.userCouponId ?? 0))
            );
            sameShop.forEach((coupon) => {
                nextUsers = nextUsers.filter((id) => id !== Number(coupon.userCouponId ?? 0));
                nextIds = nextIds.filter((id) => id !== Number(coupon.id ?? 0));
            });
            nextUsers = [...nextUsers, userCouponId];
            nextIds = [...nextIds, couponId];
        }
        void applyCoupon(nextIds, nextUsers);
    };

    const submit = async () => {
        setSubmitting(true);
        try {
            const result = await apiPost<{
                returnType?: number;
                orderId?: number | string;
                orderSn?: string;
                grouponPromotionRecordId?: number;
            }>('order/check/submit', buildPayload());
            Toast.success(t('下单成功'));
            setConfirmOpen(false);

            // returnType=2：订单不进收银台（Vue 同口径）。拼团单带着团记录 ID 时去成团详情。
            if (result?.returnType === 2) {
                if (result.grouponPromotionRecordId) {
                    router.push(`/member/group/info?id=${result.grouponPromotionRecordId}`);
                } else {
                    router.push('/member/order/list');
                }
                return;
            }
            const orderKey = result?.orderId ?? result?.orderSn ?? '';
            router.push(`/order/pay${orderKey ? `?id=${orderKey}` : ''}`);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    const coupons = useMemo(() => check?.couponList?.enableCoupons ?? [], [check]);

    /** 拼团活动（type=7）：Vue 从购物车首个条目的 activityInfo 判断，这里同口径。 */
    const grouponActivities = useMemo(() => {
        const carts = (check?.cartList ?? []).flatMap((group) => group.carts ?? []);
        const seen = new Set<number>();
        return carts
            .flatMap((item) => item.activityInfo ?? [])
            .filter((row) => {
                if (row.type !== 7) return false;
                const key = Number(row.promotionId ?? 0);
                if (seen.has(key)) return false;
                seen.add(key);
                return true;
            });
    }, [check]);
    const total = check?.total ?? {};
    const totalAmount = total.totalAmount ?? total.unpaidAmount ?? 0;

    if (needLogin) return <Empty title={t('请先登录')} description={<a href="/member/login">{t('去登录')}</a>} />;
    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('结算信息加载失败')} description={error} />;
    if (!check) return <Empty title={t('暂无待结算商品')} />;

    const current = addresses.find((item) => (item.addressId ?? item.id) === addressId);

    return (
        <div className="pc-checkout">
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

            {grouponActivities.length > 0 ? (
                <div className="pc-checkout-groupon">
                    <span className="pc-checkout-groupon-label">{t('拼团')}</span>
                    <div>
                        {grouponActivities.map((row) => (
                            <div key={row.promotionId}>
                                {row.promotionName || t('拼团活动')}
                                <span className="pc-muted">
                                    {' · '}
                                    {t('提交后将进入拼团，成团进度可在「我的拼团」查看')}
                                </span>
                            </div>
                        ))}
                    </div>
                </div>
            ) : null}

            {(check.cartList ?? []).map((group, groupIndex) => {
                const shopId = group.shopId ?? 0;
                const options = shippingMap[String(shopId)] ?? [];
                return (
                    <Card key={shopId || groupIndex} title={group.shopTitle || t('自营')}>
                        {options.length > 0 ? (
                            <div className="pc-shipping-row">
                                <span className="pc-muted">{t('配送')}</span>
                                {options.map((option) => (
                                    <button
                                        key={option.shippingTypeId}
                                        type="button"
                                        className={`pc-pay-channel${shipping[String(shopId)]?.typeId === option.shippingTypeId ? ' is-active' : ''}`}
                                        onClick={() => void chooseShipping(shopId, option)}
                                    >
                                        {option.shippingTypeName}
                                    </button>
                                ))}
                            </div>
                        ) : null}
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
                );
            })}

            <Card>
                <div className="pc-address-pick">
                    <div>
                        <div>{t('优惠券')}</div>
                        <div className="pc-muted">
                            {couponIds.length > 0 ? t('已选 {count} 张', { count: couponIds.length }) : t('未使用优惠券')}
                        </div>
                    </div>
                    <Button size="small" theme="light" disabled={coupons.length === 0} onClick={() => setPickCoupon(true)}>
                        {coupons.length === 0 ? t('暂无可用') : t('选择优惠券')}
                    </Button>
                </div>
                <div className="pc-address-pick" style={{ marginTop: 12 }}>
                    <div>
                        <div>{t('发票')}</div>
                        <div className="pc-muted">
                            {invoice.invoiceType === 0
                                ? t('不开发票')
                                : `${invoice.invoiceType === 2 ? t('增值税专用发票') : t('电子普通发票')}${
                                      invoice.companyName ? ` · ${invoice.companyName}` : ''
                                  }`}
                        </div>
                    </div>
                    <Button size="small" theme="light" onClick={() => setPickInvoice(true)}>
                        {t('修改')}
                    </Button>
                </div>
            </Card>

            <Card title={t('积分与余额')}>
                <div className="pc-address-pick">
                    <div>
                        <div>{t('使用积分')}</div>
                        <div className="pc-muted">
                            {t('本单最多可用 {points} 积分', { points: check.availablePoints ?? 0 })}
                        </div>
                    </div>
                    <Input
                        value={String(usePoint)}
                        style={{ width: 120 }}
                        onChange={(value) => {
                            const next = Math.max(0, Math.min(Number(value) || 0, check.availablePoints ?? 0));
                            setUsePoint(next);
                            void recalc({ usePoint: next });
                        }}
                    />
                </div>
                <label className="pc-address-pick" style={{ marginTop: 12, cursor: 'pointer' }}>
                    <div>
                        <div>{t('使用余额')}</div>
                        <div className="pc-muted">{t('余额可与在线支付组合使用')}</div>
                    </div>
                    <input
                        type="checkbox"
                        checked={useBalance > 0}
                        onChange={(event) => {
                            const next = event.target.checked ? 1 : 0;
                            setUseBalance(next);
                            void recalc({ useBalance: next });
                        }}
                    />
                </label>
            </Card>

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
                    <span>-{formatPrice((total.discountAmount ?? 0) + (total.couponAmount ?? 0))}</span>
                </div>
                {total.pointsAmount ? (
                    <div className="pc-amount-row">
                        <span>{t('积分抵扣')}</span>
                        <span>-{formatPrice(total.pointsAmount)}</span>
                    </div>
                ) : null}
                {total.balance ? (
                    <div className="pc-amount-row">
                        <span>{t('余额抵扣')}</span>
                        <span>-{formatPrice(total.balance)}</span>
                    </div>
                ) : null}
                <div className="pc-amount-row is-total">
                    <span>{t('应付金额')}</span>
                    <span>{formatPrice(totalAmount)}</span>
                </div>
                {recalculating ? <div className="pc-muted">{t('正在重新计价…')}</div> : null}
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

            <Modal visible={pickCoupon} title={t('选择优惠券')} onCancel={() => setPickCoupon(false)} footer={null}>
                <div className="pc-form">
                    {coupons.length === 0 ? <Empty title={t('暂无可用优惠券')} /> : null}
                    {coupons.map((coupon, index) => {
                        const userCouponId = Number(coupon.userCouponId ?? 0);
                        const active = userCouponIds.includes(userCouponId);
                        return (
                            <button
                                key={userCouponId || index}
                                type="button"
                                className={`pc-address-option${active ? ' is-active' : ''}`}
                                onClick={() => toggleCoupon(coupon)}
                            >
                                <div className="pc-addr-name">
                                    {coupon.couponDiscount
                                        ? `${coupon.couponDiscount} ${t('折')}`
                                        : formatPrice(coupon.couponMoney)}
                                    <span className="pc-muted">{coupon.couponName}</span>
                                </div>
                                <div className="pc-muted">{t('满 {amount} 可用', { amount: formatPrice(coupon.minOrderAmount) })}</div>
                            </button>
                        );
                    })}
                </div>
            </Modal>

            <InvoiceDialog
                visible={pickInvoice}
                value={invoice}
                invoiceAdded={invoiceAdded}
                mode="check"
                onClose={() => setPickInvoice(false)}
                onConfirm={(next) => {
                    setInvoice(next);
                    void recalc({ invoiceData: next });
                }}
            />

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
