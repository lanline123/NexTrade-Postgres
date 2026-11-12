import type { Metadata } from 'next';
import EntityDetail from '@/components/member/EntityDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '进度详情 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/return/LogInfo.vue */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    return <EntityDetail configKey="return/loginfo" id={searchParams?.id} />;
}
