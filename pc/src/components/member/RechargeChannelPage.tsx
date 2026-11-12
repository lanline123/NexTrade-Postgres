'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Empty, Spin, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';

/**
 * 充值支付渠道页（`/member/account/src/<channel>?id=<orderId>`）。
 * Vue 基准：pages/member/account/src/{alipay,paypal,wechat,yabanpay,yunpay,lakala}.vue
 *
 * 三种拉起方式（按 `payInfo` 里的字段判断，与 Vue 各页一致）：
 * 1. `payInfo.html`    → 第三方自动提交表单（支付宝）→ 注入 DOM 后 submit；
 * 2. `payInfo.url`     → 直接跳转（PayPal、拉卡拉微信的聚合收银台）；
 * 3. `payInfo.codeUrl` → 渲染二维码（微信 / 亚万 / 云支付 / 拉卡拉支付宝），并轮询支付状态。
 *
 * `type` 参数口径（Vue 各页写法不同，容易填错）：
 * - wechat → `wechat`；alipay → `alipay`；paypal → `paypal`
 * - 亚万 / 云支付 → `yabanpay_<wechat|alipay>` / `yunpay_<wechat|alipay>`（**下划线**）
 * - 拉卡拉 → 直接用 `payCode`（`lakalaWechat` / `lakalaAlipay`）
 *
 * 轮询：GET `user/rechargeOrder/checkStatus?id=` 每 2s 一次，最多 50 次（100s），
 * `payStatus === 1` 视为支付成功。二维码用 `qrcode` 在浏览器内生成，不依赖外部接口。
 */
type Mode = 'html' | 'redirect' | 'qr' | 'unsupported';

const CHANNEL_LABEL: Record<string, string> = {
    wechat: '微信支付',
    alipay: '支付宝',
    paypal: 'PayPal',
    yabanpay: '亚万支付',
    yunpay: '云支付',
    lakala: '拉卡拉',
};

function resolveType(channel: string, query: { type?: string; payCode?: string }): string {
    const sub = (query.type ?? '').toLowerCase();
    if (channel === 'yabanpay' || channel === 'yunpay') return `${channel}_${sub === 'alipay' ? 'alipay' : 'wechat'}`;
    if (channel === 'lakala') return query.payCode || 'lakalaWechat';
    return channel;
}

export default function RechargeChannelPage({
    channel,
    orderId,
    query,
}: {
    channel: string;
    orderId: number;
    query: { type?: string; payCode?: string };
}) {
    const { t } = useTranslation();
    const holder = useRef<HTMLDivElement | null>(null);
    const [mode, setMode] = useState<Mode>('unsupported');
    const [qr, setQr] = useState('');
    const [amount, setAmount] = useState<number | null>(null);
    const [error, setError] = useState('');
    const [loading, setLoading] = useState(true);
    const [paid, setPaid] = useState(false);
    const [expired, setExpired] = useState(false);
    const [reloadKey, setReloadKey] = useState(0);

    const label = CHANNEL_LABEL[channel] ?? channel;

    // 订单金额（拿不到不影响支付）
    useEffect(() => {
        if (!orderId) return;
        apiGet<{ order?: { amount?: string | number } }>('user/rechargeOrder/pay', { orderId })
            .then((result) => setAmount(Number(result?.order?.amount ?? 0) || null))
            .catch(() => null);
    }, [orderId]);

    // 拉起支付
    useEffect(() => {
        if (!orderId) {
            setError(t('缺少充值订单号'));
            setLoading(false);
            return;
        }
        let alive = true;
        setLoading(true);
        setError('');
        setQr('');
        (async () => {
            try {
                const result = await apiPost<{
                    payInfo?: { html?: string; url?: string; codeUrl?: string };
                }>('user/rechargeOrder/create', { id: orderId, type: resolveType(channel, query) });
                const info = result?.payInfo ?? {};
                if (!alive) return;

                if (info.html) {
                    setMode('html');
                    if (holder.current) {
                        holder.current.innerHTML = info.html;
                        holder.current.querySelector('form')?.submit();
                    }
                } else if (info.url) {
                    setMode('redirect');
                    window.location.href = info.url;
                } else if (info.codeUrl) {
                    setMode('qr');
                    const QRCode = (await import('qrcode')).default;
                    const dataUrl = await QRCode.toDataURL(info.codeUrl, { margin: 1, width: 400 });
                    if (alive) setQr(dataUrl);
                } else {
                    throw new Error(t('支付参数获取失败'));
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
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [channel, orderId, reloadKey]);

    // 扫码轮询支付状态
    useEffect(() => {
        if (mode !== 'qr' || paid || expired) return;
        let count = 0;
        const timer = setInterval(async () => {
            count += 1;
            try {
                const result = await apiGet<{ payStatus?: number }>('user/rechargeOrder/checkStatus', { id: orderId });
                if (Number(result?.payStatus) === 1) {
                    setPaid(true);
                    clearInterval(timer);
                }
            } catch {
                // 单次失败不终止，下一轮继续
            }
            if (count >= 50) {
                setExpired(true);
                clearInterval(timer);
            }
        }, 2000);
        return () => clearInterval(timer);
    }, [mode, paid, expired, orderId]);

    const retry = useCallback(() => {
        setExpired(false);
        setPaid(false);
        setReloadKey((prev) => prev + 1);
    }, []);

    if (!orderId) {
        return (
            <div className="pc-member-panel">
                <Empty title={t('缺少充值订单号')} description={<a href="/member/account/deposit">{t('返回充值')}</a>} />
            </div>
        );
    }

    if (paid) {
        return (
            <div className="pc-member-panel">
                <div className="pc-pay-result is-ok">
                    <div className="pc-pay-result-icon">✓</div>
                    <Typography.Title heading={4}>{t('充值成功')}</Typography.Title>
                    {amount ? (
                        <div className="pc-muted">
                            {t('到账金额')} {formatPrice(amount)}
                        </div>
                    ) : null}
                    <div className="pc-form-actions" style={{ justifyContent: 'center' }}>
                        <a href="/member/account/log">
                            <Button theme="solid" type="primary">
                                {t('查看充值详情')}
                            </Button>
                        </a>
                        <a href="/member/account/deposit">
                            <Button theme="borderless">{t('继续充值')}</Button>
                        </a>
                    </div>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="pc-member-panel">
                <Empty title={t('支付发起失败')} description={error} />
                <div className="pc-form-actions" style={{ justifyContent: 'center' }}>
                    <Button theme="light" onClick={retry}>
                        {t('重试')}
                    </Button>
                    <a href="/member/account/deposit">
                        <Button theme="borderless">{t('返回充值')}</Button>
                    </a>
                </div>
            </div>
        );
    }

    return (
        <div className="pc-member-panel">
            <div className="pc-pay-head">
                <Typography.Title heading={4}>{t(label)}</Typography.Title>
                {amount ? (
                    <span className="pc-muted">
                        {t('应付金额')} {formatPrice(amount)}
                    </span>
                ) : null}
            </div>

            <Spin spinning={loading}>
                {mode === 'qr' ? (
                    <div className="pc-pay-qr">
                        {qr ? (
                            <img src={qr} alt={t('支付二维码')} />
                        ) : (
                            <div className="pc-loading">{t('生成二维码中')}</div>
                        )}
                        <div className="pc-muted">
                            {expired
                                ? t('支付已超时，您可以刷新页面重新支付')
                                : t('请使用手机扫码完成支付，本页会自动刷新状态')}
                        </div>
                        {expired ? (
                            <Button theme="light" onClick={retry}>
                                {t('刷新二维码')}
                            </Button>
                        ) : null}
                    </div>
                ) : mode === 'redirect' ? (
                    <Empty
                        title={t('正在跳转到支付页面')}
                        description={t('若浏览器没有自动跳转，请点击下方按钮')}
                    />
                ) : (
                    <div ref={holder} />
                )}
            </Spin>

            {mode === 'redirect' ? (
                <div className="pc-form-actions" style={{ justifyContent: 'center' }}>
                    <Button theme="light" onClick={() => window.location.reload()}>
                        {t('重新跳转')}
                    </Button>
                    <a href="/member/account/log">
                        <Button theme="borderless">{t('我已支付')}</Button>
                    </a>
                </div>
            ) : null}
        </div>
    );
}
