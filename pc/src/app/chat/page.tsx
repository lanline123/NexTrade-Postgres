import type { Metadata } from 'next';
import Link from 'next/link';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '在线客服 · NexTrade PC' };

/**
 * 在线客服。
 * Vue 基准：pages/chat/index.vue（IM 长连接 + 表情/图片/订单卡片，体量很大）。
 * 当前为说明页：IM 通道未接，先给出可达的替代路径，避免点进来空白。
 */
export default function Page() {
    return (
        <div className="pc-placeholder">
            <h1>在线客服</h1>
            <p>IM 客服通道正在接入（依赖 WebSocket 服务与登录态）。</p>
            <p>在此之前，可通过「我的消息」留言，或在订单详情里发起售后。</p>
            <p>
                <Link href="/member/userMessage/list">我的消息</Link>
                <span> · </span>
                <Link href="/member/order/list">我的订单</Link>
                <span> · </span>
                <Link href="/">返回首页</Link>
            </p>
        </div>
    );
}
