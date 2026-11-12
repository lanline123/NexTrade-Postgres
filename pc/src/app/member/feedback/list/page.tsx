import type { Metadata } from 'next';
import FeedbackList from '@/components/member/FeedbackList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '反馈与咨询 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/feedback/List.vue、AskList.vue */
export default function Page() {
    return <FeedbackList />;
}
