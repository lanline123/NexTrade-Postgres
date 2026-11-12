import type { Metadata } from 'next';
import CatalogResults from '@/components/catalog/CatalogResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '商品列表 · NexTrade PC' };

/**
 * Vue 基准：pages/list/index.vue
 * 注意：分类走 `cat` 字段（后端不认 `categoryId`，见 lib/catalog.ts 注释）。
 */
export default function Page({
    searchParams
}: {
    searchParams?: { categoryId?: string; cat?: string; shopId?: string };
}) {
    return (
        <CatalogResults
            initialCat={searchParams?.cat ?? searchParams?.categoryId}
            shopId={Number(searchParams?.shopId) || undefined}
            emptyText="暂无商品"
        />
    );
}
