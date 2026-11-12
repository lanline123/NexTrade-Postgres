import type { Metadata } from 'next';
import JoinIntro from '@/components/join/JoinIntro';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '商家入驻 · NexTrade PC' };

/** Vue 基准：pages/join/index.vue、intro.vue */
export default function Page() {
    return <JoinIntro />;
}
