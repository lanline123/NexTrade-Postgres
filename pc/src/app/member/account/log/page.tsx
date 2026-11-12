import type { Metadata } from 'next';
import AccountLog from '@/components/member/AccountLog';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '申请记录 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/account/Log.vue */
export default function Page() {
    return <AccountLog />;
}
