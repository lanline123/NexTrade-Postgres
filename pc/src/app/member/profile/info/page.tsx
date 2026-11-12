import type { Metadata } from 'next';
import ProfileForm from '@/components/member/ProfileForm';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '个人资料 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/profile/Info.vue */
export default function Page() {
    return <ProfileForm />;
}
