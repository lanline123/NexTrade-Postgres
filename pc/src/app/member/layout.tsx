import type { ReactNode } from 'react';
import MemberShell from '@/components/MemberShell';

export const metadata = {
    title: '会员中心 · NexTrade PC'
};

/** /member/** 共用外壳；子页迁移时只改各自 page.tsx，外壳不动。 */
export default function MemberLayout({ children }: { children: ReactNode }) {
    return <MemberShell>{children}</MemberShell>;
}
