import type { Metadata } from 'next';
import JoiningApply from '@/components/join/JoiningApply';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的入驻申请 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/joining/List.vue（merchant/merchant/myApply） */
export default function Page() {
    return <JoiningApply />;
}
