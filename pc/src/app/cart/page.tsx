import type { Metadata } from 'next';
import CartCore from '@/components/cart/CartCore';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '购物车 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/cart/index.vue（2249 行，本页为核心链路） */
export default function Page() {
    return <CartCore />;
}
