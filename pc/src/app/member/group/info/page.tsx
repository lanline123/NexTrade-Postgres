import type { Metadata } from 'next';
import EntityDetail from '@/components/member/EntityDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '团购详情 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/group/Info.vue */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    return <EntityDetail configKey="group/info" id={searchParams?.id} />;
}
