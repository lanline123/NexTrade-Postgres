import type { Metadata } from 'next';
import PcHeader from '@/components/PcHeader';
import PcFooter from '@/components/PcFooter';
import GiftClaimClient from '@/components/GiftClaimClient';
import { getPcHeader } from '@/lib/api';

import { getServerT, serverDecorateLocaleCode } from '@/i18n/server';
export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '领取礼物 · 万物新生', robots: { index: false, follow: false } };

/**
 * 礼物领取页（我方新增，路由 /gift/claim?token=）。
 * 买家付款后分享领取链接，好友在此登录并填自己的地址领取；
 * 页面不显示任何价格（送礼语义）。
 */
export default async function GiftClaimPage() {
  const t = getServerT();
  const header = await getPcHeader(undefined, serverDecorateLocaleCode()).catch(() => null);
  return (
    <>
      <PcHeader config={header as any} />
      <main className="pc-member-page">
        <div className="pc-container" style={{ maxWidth: 900, margin: '24px auto' }}>
          <h1 className="pc-cart-title">{t('领取礼物')}</h1>
          <GiftClaimClient />
        </div>
      </main>
      <PcFooter />
    </>
  );
}
