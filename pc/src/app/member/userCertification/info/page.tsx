import type { Metadata } from 'next';
import CertificationPanel from '@/components/member/CertificationPanel';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '实名认证 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/userCertification/Info.vue + src/step.vue */
export default function Page() {
    return <CertificationPanel />;
}
