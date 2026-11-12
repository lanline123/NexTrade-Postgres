import type { Metadata } from 'next';
import CardManagement from '@/components/member/CardManagement';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '卡管理 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/account/CardManagement.vue（user/withdrawApply/*） */
export default function Page() {
    return <CardManagement />;
}
