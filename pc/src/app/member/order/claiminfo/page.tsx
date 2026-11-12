import type { Metadata } from 'next';
import EntityDetail from '@/components/member/EntityDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '线下支付认款 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/order/claimInfo.vue */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    return <EntityDetail configKey="order/claiminfo" id={searchParams?.id} />;
}
