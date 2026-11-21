import type { Metadata } from 'next';
import PcHeader from '@/components/PcHeader';
import MemberShell from '@/components/MemberShell';
import GiftReceivedClient from '@/components/GiftReceivedClient';
import { getPcHeader } from '@/lib/api';
import PcFooter from '@/components/PcFooter';

import { getServerT, serverDecorateLocaleCode } from '@/i18n/server';
export const dynamic = 'force-dynamic';

export async function generateMetadata(): Promise<Metadata> {
  const t = getServerT();
  return {
    title: t('我收到的礼物 · 万物新生'),
    robots: { index: false, follow: false },
  };
}

/** 我收到的礼物（我方新增）：领取人视角，无价格展示。 */
export default async function MemberGiftPage() {
  const t = getServerT();
  const header = await getPcHeader(undefined, serverDecorateLocaleCode()).catch(() => null);
  return (
    <>
      <PcHeader config={header as any} />
      <main className="pc-member-page">
        <MemberShell title={t('我收到的礼物')}>
          <GiftReceivedClient />
        </MemberShell>
      </main>
      <PcFooter />
    </>
  );
}
