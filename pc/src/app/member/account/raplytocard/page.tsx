import type { Metadata } from 'next';
import WithdrawPage from '@/components/member/WithdrawPage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '余额提现 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/account/Raplytocard.vue（user/withdrawApply/apply） */
export default function Page() {
    return <WithdrawPage />;
}
