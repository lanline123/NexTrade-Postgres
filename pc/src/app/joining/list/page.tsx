import type { Metadata } from 'next';
import JoiningList from '@/components/join/JoiningList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '入驻申请 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/joining/List.vue */
export default function Page() {
    return <JoiningList />;
}
