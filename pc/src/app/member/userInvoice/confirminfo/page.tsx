import type { Metadata } from 'next';
import EntityDetail from '@/components/member/EntityDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '确认发票信息 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/userInvoice/confirmInfo.vue */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    return <EntityDetail configKey="userInvoice/confirminfo" id={searchParams?.id} />;
}
