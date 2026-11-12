import type { Metadata } from 'next';
import AuthPanel from '@/components/auth/AuthPanel';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '注册 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/Register.vue */
export default function Page() {
    return <AuthPanel mode="register" />;
}
