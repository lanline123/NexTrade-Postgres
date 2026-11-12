import type { Metadata } from 'next';
import ProductResults from '@/components/catalog/ProductResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店内搜索 · NexTrade PC' };

/** Vue 基准：pages/shop/search/[id].vue（shops/search/src/ShopSearchLayoutModern.vue） */
export default function Page({ params, searchParams }: { params: { id?: string }; searchParams?: { keyword?: string } }) {
    const shopId = Number(params.id) || undefined;
    return <ProductResults shopId={shopId} keyword={searchParams?.keyword ?? ''} emptyText="店内没有找到相关商品" />;
}
