import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '还款卡管理 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/account/Raplytocard.vue
 * 账期支付本身也尚未迁移（结算页 useCredit 固定 0）。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>还款卡管理</h1>
            <p>还款卡用于账期支付还款，依赖账期与支付渠道，待账期支付接入后开放。</p>
            <p>
                <Link href="/member">返回会员中心</Link>
                <span> · </span>
                <Link href="/member/termPay/list">查看账期明细</Link>
            </p>
        </div>
    );
}
