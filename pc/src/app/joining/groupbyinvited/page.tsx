import type { Metadata } from 'next';
import GroupInvitePage from '@/components/join/GroupInvitePage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '拼团邀请 · NexTrade PC' };

/** Vue 基准：pages/joining/groupByInvited.vue（`?id=<grouponPromotionRecordId>`） */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    return <GroupInvitePage recordId={Number(searchParams?.id) || 0} />;
}
