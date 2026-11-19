import { notFound } from 'next/navigation';
import { getAuctionDetail } from '@/lib/api';
import AuctionDetailClient from '@/components/AuctionDetailClient';

export const dynamic = 'force-dynamic';

export default async function AuctionDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const itemId = Number(id);
  if (!Number.isFinite(itemId) || itemId <= 0) notFound();
  const detail = await getAuctionDetail(itemId).catch(() => null);
  if (!detail) notFound();
  return (
    <main className="auction-page container">
      <AuctionDetailClient itemId={itemId} initial={detail} />
    </main>
  );
}
