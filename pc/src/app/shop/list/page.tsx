import type { Metadata } from 'next';
import ShopList from '@/components/shop/ShopList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '店铺 · NexTrade PC' };

/** Vue 基准：pages/shop/List.vue */
export default function Page() {
    return <ShopList />;
}
