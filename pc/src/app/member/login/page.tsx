import type { Metadata } from 'next';
import AuthPanel from '@/components/auth/AuthPanel';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '登录 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/Login.vue */
export default function Page() {
    return <AuthPanel mode="login" />;
}
