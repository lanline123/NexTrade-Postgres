'use client';

import { useEffect, useState } from 'react';
import { Button, Card, Empty, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';

/**
 * 收银台（只读版）。
 * Vue 基准：pages/order/pay.vue（842 行）+ 各渠道页（alipay/wechat/paypal/...）。
 * 当前只做：取 order/pay/index 展示订单与可用渠道、选择渠道。
 * 刻意不调 order/pay/create —— 那会真实发起支付，等支付联调时再接，避免误扣款。
 */
interface PayOrder {
    orderSn?: string;
    totalAmount?: string | number;
    unpaidAmount?: string | number;
    orderStatusName?: string;
}

interface PayInfo {
    order?: PayOrder;
    paymentList?: string[];
}

const PAY_LABELS: Record<string, string> = {
    alipay: '支付宝',
    wechat: '微信支付',
    wechatpay: '微信支付',
    balance: '余额支付',
    paypal: 'PayPal',
    stripe: 'Stripe',
    huifu: '汇付',
    lakala: '拉卡拉',
    yeepay: '易宝支付',
    yunpay: '云支付',
    yabanpay: '亚班支付',
    offline: '线下支付'
};

export default function PayCore({ orderId }: { orderId: string }) {
    const { t } = useTranslation();
    const [info, setInfo] = useState<PayInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [channel, setChannel] = useState('');

    useEffect(() => {
        if (!orderId) {
            setError(t('缺少订单号'));
            setLoading(false);
            return;
        }
        apiGet<PayInfo>('order/pay/index', { id: orderId })
            .then((result) => {
                setInfo(result);
                setChannel(result.paymentList?.[0] ?? '');
            })
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [orderId, t]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('支付信息加载失败')} description={error} />;
    if (!info?.order) return <Empty title={t('暂无待支付订单')} />;

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={3}>{t('收银台')}</Typography.Title>

            <Card>
                <div className="pc-amount-row">
                    <span>{t('订单号')}</span>
                    <span>{info.order.orderSn ?? orderId}</span>
                </div>
                <div className="pc-amount-row is-total">
                    <span>{t('应付金额')}</span>
                    <span>{formatPrice(info.order.unpaidAmount ?? info.order.totalAmount)}</span>
                </div>
            </Card>

            <Card title={t('支付方式')}>
                {(info.paymentList ?? []).length === 0 ? (
                    <Empty title={t('无可用的支付方式')} />
                ) : (
                    <div className="pc-pay-channels">
                        {(info.paymentList ?? []).map((code) => (
                            <button
                                key={code}
                                type="button"
                                className={`pc-pay-channel${channel === code ? ' is-active' : ''}`}
                                onClick={() => setChannel(code)}
                            >
                                {PAY_LABELS[code] ?? code}
                            </button>
                        ))}
                    </div>
                )}
                <div className="pc-muted" style={{ marginTop: 12 }}>
                    {t('支付渠道接入中：当前只展示可选方式，不会发起真实支付。')}
                </div>
            </Card>

            <div className="pc-order-foot">
                <span />
                <Button
                    theme="solid"
                    type="primary"
                    disabled
                    onClick={() => Toast.info(t('支付渠道接入中'))}
                >
                    {t('去支付')}
                </Button>
            </div>
        </div>
    );
}
