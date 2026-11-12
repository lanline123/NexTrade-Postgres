import type { Metadata } from 'next';
import GiftCardExchange from '@/components/member/GiftCardExchange';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '礼品卡兑换 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/giftCard/exchange.vue */
export default function Page() {
    return <GiftCardExchange />;
}
