import type { Metadata } from 'next';
import CollectProductList from '@/components/member/CollectProductList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '收藏商品 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/collectProduct/List.vue */
export default function Page() {
    return <CollectProductList />;
}
