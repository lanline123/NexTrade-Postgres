import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '编辑收货地址 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/address/EditRegion.vue
 * 列表页支持新增与删除；完整地区级联选择待接区域接口。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>编辑收货地址</h1>
            <p>地址新增/编辑已合并到收货地址列表页的表单，此路径保留为入口说明。</p>
            <p>
                <Link href="/member">返回会员中心</Link>
                <span> · </span>
                <Link href="/member/address/list">去收货地址</Link>
            </p>
        </div>
    );
}
