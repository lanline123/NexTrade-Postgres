import type { Metadata } from 'next';
import ShopHome from '@/components/shop/ShopHome';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店铺主页 · NexTrade PC' };

/** Vue 基准：pages/shop/[id].vue */
export default function Page({ params }: { params: { id?: string } }) {
    const shopId = Number(params.id) || 0;
    return <ShopHome shopId={shopId} />;
}
