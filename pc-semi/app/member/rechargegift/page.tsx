import type { Metadata } from 'next';
import PcHeader from '@/components/PcHeader';
import RechargeGiftClient from '@/components/RechargeGiftClient';
import { getPcHeader } from '@/lib/api';
import PcFooter from '@/components/PcFooter';
import MemberShell from '@/components/MemberShell';

import { getServerT, serverDecorateLocaleCode } from '@/i18n/server';
export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的赠品 · 万物新生', robots: { index: false, follow: false } };

export default async function RechargeGiftPage() {
    const t = getServerT();
  const header = await getPcHeader(undefined, serverDecorateLocaleCode()).catch(() => null);
  return (
    <>
      <PcHeader config={header as any} />
      <main className="pc-member-page">
        <MemberShell title={t('我的赠品')}>
    <RechargeGiftClient />
        </MemberShell>
      </main>
      <PcFooter />
    </>
  );
}
