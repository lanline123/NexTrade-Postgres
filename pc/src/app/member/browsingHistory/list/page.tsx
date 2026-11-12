import type { Metadata } from 'next';
import HistoryList from '@/components/member/HistoryList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '浏览记录 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/browsingHistory/List.vue */
export default function Page() {
    return <HistoryList />;
}
