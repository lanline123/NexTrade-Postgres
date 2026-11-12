import type { Metadata } from 'next';
import ProductResults from '@/components/catalog/ProductResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店铺分类 · NexTrade PC' };

/** Vue 基准：pages/shopCategory/[id].vue（复用商品网格，筛选栏未迁） */
export default function Page({ params }: { params: { id?: string } }) {
    const categoryId = Number(params.id) || undefined;
    return <ProductResults categoryId={categoryId} emptyText="该分类暂无商品" />;
}
