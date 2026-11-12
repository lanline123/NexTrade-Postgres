import type { Metadata } from 'next';
import CatalogResults from '@/components/catalog/CatalogResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店铺分类 · NexTrade PC' };

/**
 * Vue 基准：pages/shopCategory/[id].vue（src/ShopCategory.vue）
 * 店铺分类字段是 `shopCategoryId`，不是平台分类的 `cat`；店铺内不展示平台筛选栏。
 */
export default function Page({ params, searchParams }: { params: { id?: string }; searchParams?: { categoryId?: string } }) {
    return (
        <CatalogResults
            shopId={Number(params.id) || undefined}
            shopCategoryId={Number(searchParams?.categoryId) || undefined}
            showFilter={false}
            emptyText="该分类暂无商品"
        />
    );
}
