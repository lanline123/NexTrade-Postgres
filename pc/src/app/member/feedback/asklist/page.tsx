import type { Metadata } from 'next';
import FeedbackList from '@/components/member/FeedbackList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '咨询记录 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/feedback/AskList.vue（复用已有列表组件） */
export default function Page() {
    return <FeedbackList />;
}
