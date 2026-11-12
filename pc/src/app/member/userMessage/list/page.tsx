import type { Metadata } from 'next';
import MessageList from '@/components/member/MessageList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的消息 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/userMessage/List.vue */
export default function Page() {
    return <MessageList />;
}
