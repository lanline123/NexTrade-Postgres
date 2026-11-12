import type { Metadata } from 'next';
import PinList from '@/components/member/PinList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的团购 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/pin/List.vue */
export default function Page() {
    return <PinList />;
}
