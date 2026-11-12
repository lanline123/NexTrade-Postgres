import type { Metadata } from 'next';
import CatalogResults from '@/components/catalog/CatalogResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '搜索 · NexTrade PC' };

/** Vue 基准：pages/search/components/search.vue（筛选栏 + 结果流） */
export default function Page({
    searchParams
}: {
    searchParams?: { keyword?: string; shopId?: string; cat?: string };
}) {
    return (
        <CatalogResults
            keyword={searchParams?.keyword ?? ''}
            initialCat={searchParams?.cat}
            shopId={Number(searchParams?.shopId) || undefined}
            emptyText="没有找到相关商品"
        />
    );
}
