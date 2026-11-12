import type { Metadata } from 'next';
import CollectShopList from '@/components/member/CollectShopList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '收藏店铺 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/collectShop/List.vue */
export default function Page() {
    return <CollectShopList />;
}
