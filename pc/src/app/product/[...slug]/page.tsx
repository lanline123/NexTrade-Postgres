import type { Metadata } from 'next';
import ProductDetail from '@/components/product/ProductDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '商品详情 · NexTrade PC' };

/** Vue 基准：pages/product/[...slug].vue（SEO 路径，slug 可能是货号或 ID） */
export default function Page({ params }: { params: { slug?: string[] } }) {
    const idOrSn = (params.slug ?? [])[0] ?? '';
    return <ProductDetail idOrSn={decodeURIComponent(idOrSn)} />;
}
