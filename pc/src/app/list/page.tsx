import type { Metadata } from 'next';
import ProductResults from '@/components/catalog/ProductResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '商品列表 · NexTrade PC' };

/** Vue 基准：pages/list/index.vue */
export default function Page({ searchParams }: { searchParams?: { categoryId?: string; shopId?: string } }) {
    const categoryId = Number(searchParams?.categoryId) || undefined;
    const shopId = Number(searchParams?.shopId) || undefined;
    return <ProductResults categoryId={categoryId} shopId={shopId} emptyText="暂无商品" />;
}
