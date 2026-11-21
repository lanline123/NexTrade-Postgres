import type { Metadata } from 'next';
import PcHeader from '@/components/PcHeader';
import PcFooter from '@/components/PcFooter';
import GiftCardSelectClient from '@/components/GiftCardSelectClient';
import { getPcHeader } from '@/lib/api';

import { getServerT, serverDecorateLocaleCode } from '@/i18n/server';
export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '礼品卡选品 · 万物新生', robots: { index: false, follow: false } };

/**
 * 兑换卡任选选品页（原版有，路由 /giftcard/select）。
 * 入参 giftCardsId / giftCardSn（详情页「去使用」任选分流跳入）。
 */
export default async function GiftCardSelectPage() {
  const t = getServerT();
  const header = await getPcHeader(undefined, serverDecorateLocaleCode()).catch(() => null);
  return (
    <>
      <PcHeader config={header as any} />
      <main className="pc-member-page">
        <div className="pc-container" style={{ maxWidth: 900, margin: '24px auto' }}>
          <h1 className="pc-cart-title">{t('礼品卡选品')}</h1>
          <GiftCardSelectClient />
        </div>
      </main>
      <PcFooter />
    </>
  );
}
