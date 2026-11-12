import type { Metadata } from 'next';
import DepositPage from '@/components/member/DepositPage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '余额充值 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/account/Deposit.vue（user/rechargeOrder/*） */
export default function Page() {
    return <DepositPage />;
}
