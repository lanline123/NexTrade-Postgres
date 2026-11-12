import type { Metadata } from 'next';
import AccountDetail from '@/components/member/AccountDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '账户余额 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/account/Detail.vue */
export default function Page() {
    return <AccountDetail />;
}
