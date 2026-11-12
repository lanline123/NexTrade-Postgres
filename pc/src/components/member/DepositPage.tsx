'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Empty, Input, Radio, RadioGroup, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';

/**
 * 会员余额充值。
 * Vue 基准：pages/member/account/Deposit.vue（419 行）
 *
 * 真实接口：
 * - GET  user/rechargeOrder/setting        → 充值档位 records[{ rechargeId, money, discountMoney, isShow }]
 * - GET  user/rechargeOrder/paymentList    → 可用支付方式（本环境只返回 ["alipay"]）
 * - POST user/rechargeOrder/update         → 创建充值订单，返回 { orderId }
 * - 之后跳 `/member/account/src/<channel>/index?id=<orderId>`，由该渠道页调
 *   `user/rechargeOrder/create { id, type }` 拿到 `payInfo.html`（自动提交的表单）完成跳转。
 *
 * 选择的档位会被回报 `rechargeId`（0 表示自定义金额）。支付方式取后端列表，不硬编码渠道。
 */
interface Tier {
    rechargeId?: number;
    money?: string | number;
    discountMoney?: string | number;
    isShow?: number;
}

const CHANNEL_LABEL: Record<string, string> = {
    wechat: '微信支付',
    alipay: '支付宝',
    paypal: 'PayPal',
    yabanpayWechat: '亚万支付（微信）',
    yabanpayAlipay: '亚万支付（支付宝）',
    yunpayWechat: '云支付（微信）',
    yunpayAlipay: '云支付（支付宝）',
    lakalaWechat: '拉卡拉（微信）',
    lakalaAlipay: '拉卡拉（支付宝）'
};

/** 渠道 code → 渠道页路径（Vue 的 7 个支付渠道页在 `pages/member/account/src/` 下）。 */
function channelPath(channel: string): string {
    if (channel.startsWith('yabanpay')) return `/member/account/src/yabanpay?type=${channel.replace('yabanpay', '').toLowerCase()}`;
    if (channel.startsWith('yunpay')) return `/member/account/src/yunpay?type=${channel.replace('yunpay', '').toLowerCase()}`;
    if (channel.startsWith('lakala')) return `/member/account/src/lakala?payCode=${channel}`;
    return `/member/account/src/${channel}`;
}

export default function DepositPage() {
    const { t } = useTranslation();
    const [tiers, setTiers] = useState<Tier[]>([]);
    const [channels, setChannels] = useState<string[]>([]);
    const [rechargeId, setRechargeId] = useState(0);
    const [amount, setAmount] = useState('');
    const [channel, setChannel] = useState('');
    const [loading, setLoading] = useState(true);
    const [submitting, setSubmitting] = useState(false);

    useEffect(() => {
        (async () => {
            try {
                const [setting, payList] = await Promise.all([
                    apiGet<{ records?: Tier[] }>('user/rechargeOrder/setting').catch(() => ({ records: [] })),
                    apiGet<string[]>('user/rechargeOrder/paymentList').catch(() => []),
                ]);
                const list = (setting.records ?? []).filter((row) => row.isShow !== 0);
                setTiers(list);
                setChannels(Array.isArray(payList) ? payList : []);
                setChannel(Array.isArray(payList) && payList.length ? payList[0] : '');
            } finally {
                setLoading(false);
            }
        })();
    }, []);

    const pickTier = useCallback((tier: Tier) => {
        setRechargeId(Number(tier.rechargeId ?? 0));
        setAmount(String(tier.money ?? ''));
    }, []);

    const submit = async () => {
        if (!channel) return Toast.warning(t('暂无可用的支付方式'));
        const value = Number(amount);
        if (!amount || !Number.isFinite(value) || value <= 0) return Toast.warning(t('请输入正确的充值金额'));

        setSubmitting(true);
        try {
            const result = await apiPost<{ orderId?: number | string }>('user/rechargeOrder/update', {
                id: rechargeId,
                amount: value,
                money: value,
                payType: channel
            });
            const orderId = result?.orderId ?? '';
            if (!orderId) throw new Error(t('充值订单创建失败'));
            // 渠道页负责真正拉起支付（表单跳转或扫码）
            window.open(`${channelPath(channel)}${channelPath(channel).includes('?') ? '&' : '?'}id=${orderId}`, '_blank');
            Toast.success(t('充值订单已创建，请在新窗口完成支付'));
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setSubmitting(false);
        }
    };

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('余额充值')}</Typography.Title>
            </div>

            {tiers.length === 0 ? (
                <Empty title={t('暂无充值档位')} />
            ) : (
                <div className="pc-form">
                    <div>
                        <span className="pc-form-label">{t('选择充值金额')}</span>
                        <div className="pc-filter-values">
                            {tiers.map((tier) => (
                                <span
                                    key={tier.rechargeId}
                                    className={`pc-filter-chip${rechargeId === Number(tier.rechargeId) ? ' is-active' : ''}`}
                                    onClick={() => pickTier(tier)}
                                >
                                    {formatPrice(tier.money)}
                                    {Number(tier.discountMoney) > 0 ? (
                                        <em className="pc-filter-count">
                                            {t('送')}
                                            {formatPrice(tier.discountMoney)}
                                        </em>
                                    ) : null}
                                </span>
                            ))}
                        </div>
                    </div>

                    <div>
                        <span className="pc-form-label">{t('或输入自定义金额')}</span>
                        <Input
                            value={amount}
                            placeholder={t('充值金额')}
                            onChange={(v) => {
                                setAmount(String(v).replace(/[^\d.]/g, ''));
                                setRechargeId(0);
                            }}
                        />
                    </div>

                    <div>
                        <span className="pc-form-label">{t('支付方式')}</span>
                        {channels.length === 0 ? (
                            <div className="pc-muted">{t('暂无可用的支付方式')}</div>
                        ) : (
                            <RadioGroup value={channel} onChange={(e) => setChannel(String(e.target.value))}>
                                {channels.map((item) => (
                                    <Radio key={item} value={item}>
                                        {t(CHANNEL_LABEL[item] ?? item)}
                                    </Radio>
                                ))}
                            </RadioGroup>
                        )}
                    </div>

                    <div className="pc-form-actions">
                        <Button
                            theme="solid"
                            type="primary"
                            loading={submitting}
                            disabled={channels.length === 0}
                            onClick={() => void submit()}
                        >
                            {t('去支付')}
                        </Button>
                        <a href="/member/account/detail">
                            <Button theme="borderless">{t('账户明细')}</Button>
                        </a>
                    </div>

                    <div className="pc-note">
                        {t('充值完成后余额会体现在账户明细中；发票与退款请联系平台客服。')}
                    </div>
                </div>
            )}

            <Card className="pc-muted" style={{ marginTop: 16, fontSize: 12 }}>
                {t('说明：微信 / 亚万 / 云支付 / 拉卡拉等渠道页需要扫码展示，尚未迁移；当前仅支付宝、PayPal 走跳转表单。')}
            </Card>
        </div>
    );
}
