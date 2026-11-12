import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '充值服务协议 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/account/src/serviceAgreement.vue
 * 协议正文由运营在后台维护（Vue 版是静态长文），这里给出入口说明，正文沿用《用户服务协议》。
 */
export default function Page() {
    return (
        <div className="pc-member-panel">
            <h1>充值服务协议</h1>
            <p className="pc-muted">
                充值即视为同意平台《用户服务协议》与《余额服务规则》。余额仅可用于本平台消费，不支持提现到非本人账户。
            </p>
            <p>
                <Link href="/article/issue/index">查看平台协议与规则</Link>
                <span> · </span>
                <Link href="/member/account/deposit">返回充值</Link>
            </p>
        </div>
    );
}
