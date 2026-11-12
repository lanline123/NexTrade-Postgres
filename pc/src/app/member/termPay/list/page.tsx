import type { Metadata } from 'next';
import CreditList from '@/components/member/CreditList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '账期明细 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/termPay/list.vue */
export default function Page() {
    return <CreditList />;
}
