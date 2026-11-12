import type { Metadata } from 'next';
import ChatPage from '@/components/chat/ChatPage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '在线客服 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/chat/index.vue（IM 会话 + 消息 + WebSocket） */
export default function Page() {
    return <ChatPage />;
}
