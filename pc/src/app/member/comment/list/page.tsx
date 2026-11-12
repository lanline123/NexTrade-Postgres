import type { Metadata } from 'next';
import CommentList from '@/components/member/CommentList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的评价 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/comment/List.vue */
export default function Page() {
    return <CommentList />;
}
