import type { Metadata } from 'next';
import CatalogResults from '@/components/catalog/CatalogResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店内搜索 · NexTrade PC' };

/** Vue 基准：pages/shop/search/[id].vue（shops/search/src/ShopSearchLayoutModern.vue） */
export default function Page({ params, searchParams }: { params: { id?: string }; searchParams?: { keyword?: string } }) {
    const shopId = Number(params.id) || undefined;
    return (
        <CatalogResults
            shopId={shopId}
            keyword={searchParams?.keyword ?? ''}
            // 店内筛选是另一套 ShopFilter（getFilter 不认 shopId，传了会 500），这里只出结果
            showFilter={false}
            emptyText="店内没有找到相关商品"
        />
    );
}
