import type { Metadata } from 'next';
import ImageSearchPage from '@/components/catalog/ImageSearchPage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '以图搜图 · NexTrade PC' };

/** Vue 基准：pages/search/image.vue + components/imageSearch/* */
export default function Page() {
    return <ImageSearchPage />;
}
