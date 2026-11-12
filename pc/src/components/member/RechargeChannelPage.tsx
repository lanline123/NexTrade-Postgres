'use client';

import { useEffect, useRef, useState } from 'react';
import { Empty, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';

/**
 * 充值支付渠道页（`/member/account/src/<channel>?id=<orderId>`）。
 * Vue 基准：pages/member/account/src/{alipay,paypal}.vue（各 32–35 行）
 *
 * 流程：POST `user/rechargeOrder/create { id, type }` → `payInfo.html`（一段自动提交的
 * 支付宝/PayPal 表单）→ 原样注入 DOM 并 submit，浏览器跳到第三方收银台。
 *
 * 未迁的渠道：wechat / yabanpay / yunpay / lakala —— 这几个 Vue 页面是**扫码**（要渲染二维码），
 * 需要引入 qrcode 依赖，本次先给出明确提示，不静默失败。
 */
const FORM_CHANNELS = ['alipay', 'paypal'];

const QR_CHANNELS: Record<string, string> = {
    wechat: '微信支付',
    yabanpay: '亚万支付',
    yunpay: '云支付',
    lakala: '拉卡拉',
};

export default function RechargeChannelPage({
    channel,
    orderId,
}: {
    channel: string;
    orderId: number;
    payCode?: string;
}) {
    const { t } = useTranslation();
    const holder = useRef<HTMLDivElement | null>(null);
    const [error, setError] = useState('');
    const [loading, setLoading] = useState(FORM_CHANNELS.includes(channel));

    useEffect(() => {
        if (!FORM_CHANNELS.includes(channel)) return;
        if (!orderId) {
            setError(t('缺少充值订单号'));
            setLoading(false);
            return;
        }
        let alive = true;
        (async () => {
            try {
                const result = await apiPost<{ payInfo?: { html?: string } }>('user/rechargeOrder/create', {
                    id: orderId,
                    type: channel,
                });
                const html = result?.payInfo?.html ?? '';
                if (!alive) return;
                if (!html) throw new Error(t('支付参数获取失败'));
                // 第三方给的是一段带 auto-submit 的表单，注入后手动提交
                if (holder.current) {
                    holder.current.innerHTML = html;
                    const form = holder.current.querySelector('form');
                    form?.submit();
                }
            } catch (e) {
                if (alive) setError((e as Error).message);
            } finally {
                if (alive) setLoading(false);
            }
        })();
        return () => {
            alive = false;
        };
    }, [channel, orderId, t]);

    if (QR_CHANNELS[channel]) {
        return (
            <div className="pc-member-panel">
                <Empty
                    title={t('该渠道的扫码支付页尚未迁移')}
                    description={`${t(QR_CHANNELS[channel])}${t('需要在页面内渲染二维码，暂未接入')}`}
                />
                <div className="pc-note">
                    {t('充值订单已创建，单号')}：{orderId}
                    {t('，可更换为支付宝支付，或稍后在账户明细中查看状态。')}
                    <a href="/member/account/deposit"> {t('返回充值')}</a>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="pc-member-panel">
                <Empty title={t('支付发起失败')} description={error} />
                <div className="pc-note">
                    <a href="/member/account/deposit">{t('返回充值')}</a>
                </div>
            </div>
        );
    }

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>
                {loading ? t('正在跳转到支付页面') : t('支付页面')}
            </Typography.Title>
            <div ref={holder} />
        </div>
    );
}
