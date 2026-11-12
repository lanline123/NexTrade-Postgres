import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '银行卡管理 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/account/CardManagement.vue
 * 当前为说明页，不提供假绑定入口。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>银行卡管理</h1>
            <p>银行卡绑定/解绑依赖支付渠道的签约接口，待支付中心接入后开放。</p>
            <p>
                <Link href="/member">返回会员中心</Link>
                <span> · </span>
                <Link href="/member/account/log">查看账户流水</Link>
            </p>
        </div>
    );
}
