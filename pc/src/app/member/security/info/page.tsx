import type { Metadata } from 'next';
import SecurityPanel from '@/components/member/SecurityPanel';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '账号安全 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/security/Info.vue + src/VerificationSecurity.vue */
export default function Page() {
    return <SecurityPanel />;
}
