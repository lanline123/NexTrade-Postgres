import type { Metadata } from 'next';
import AddressEditPage from '@/components/member/AddressEditPage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '编辑收货地址 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/address/EditRegion.vue
 * 路由约定与 Vue 一致：`?act=add` 新增、`?act=edit&addressId=x` 编辑。
 */
export default function Page({ searchParams }: { searchParams?: { act?: string; addressId?: string; id?: string } }) {
    const addressId = Number(searchParams?.addressId ?? searchParams?.id) || 0;
    return <AddressEditPage addressId={addressId} act={searchParams?.act} />;
}
