import type { Metadata } from 'next';
import ProductResults from '@/components/catalog/ProductResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '搜索 · NexTrade PC' };

/** Vue 基准：pages/search/components/search.vue（筛选栏尚未迁移） */
export default function Page({ searchParams }: { searchParams?: { keyword?: string; shopId?: string } }) {
    const keyword = searchParams?.keyword ?? '';
    const shopId = Number(searchParams?.shopId) || undefined;
    return <ProductResults keyword={keyword} shopId={shopId} emptyText="没有找到相关商品" />;
}
