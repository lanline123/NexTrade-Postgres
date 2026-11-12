import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '邀请团购 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * 邀请团购。
 * Vue 基准：pages/joining/groupByInvited.vue（961 行，含邀请关系、成团进度、拼团商品）。
 * 当前为说明页：邀请关系与成团进度依赖拼团活动接口，等拼团模块整体迁移后补齐。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>邀请团购</h1>
            <p>邀请关系与成团进度依赖拼团活动模块，正在迁移中。</p>
            <p>可以先在我的团购里查看已参与的团。</p>
            <p>
                <Link href="/member/pin/list">我的团购</Link>
                <span> · </span>
                <Link href="/activity">活动专题</Link>
                <span> · </span>
                <Link href="/">返回首页</Link>
            </p>
        </div>
    );
}
