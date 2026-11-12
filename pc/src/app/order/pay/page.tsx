import type { Metadata } from 'next';
import PayCore from '@/components/order/PayCore';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '收银台 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/order/pay.vue（842 行，本页为只读展示版） */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    const orderId = searchParams?.id ?? '';
    return <PayCore orderId={orderId} />;
}
