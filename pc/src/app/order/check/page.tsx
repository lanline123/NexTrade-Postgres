import type { Metadata } from 'next';
import CheckoutCore from '@/components/order/CheckoutCore';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '确认订单 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/order/check.vue（1679 行，本页为先做核心链路） */
export default function Page({ searchParams }: { searchParams?: { flowType?: string } }) {
    const flowType = Number(searchParams?.flowType) || 1;
    return <CheckoutCore flowType={flowType} />;
}
