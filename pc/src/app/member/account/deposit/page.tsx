import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '账户充值 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/account/Deposit.vue
 * 充值下单（rechargeOrder/create）尚未接入，避免产生未支付充值单。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>账户充值</h1>
            <p>充值面额与支付渠道需与支付中心联调，当前请到账户余额页查看余额与流水。</p>
            <p>
                <Link href="/member">返回会员中心</Link>
                <span> · </span>
                <Link href="/member/account/detail">查看账户余额</Link>
            </p>
        </div>
    );
}
