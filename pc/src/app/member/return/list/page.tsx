import type { Metadata } from 'next';
import AfterSalesList from '@/components/member/AfterSalesList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '售后退款 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/return/List.vue */
export default function Page() {
    return <AfterSalesList />;
}
