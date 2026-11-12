import type { Metadata } from 'next';
import EntityDetail from '@/components/member/EntityDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '拼团详情 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/pin/Group.vue */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    return <EntityDetail configKey="pin/group" id={searchParams?.id} />;
}
