import type { Metadata } from 'next';
import ExchangeList from '@/components/catalog/ExchangeList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '换购专区 · NexTrade PC' };

/** Vue 基准：pages/exchange/List.vue */
export default function Page() {
    return <ExchangeList />;
}
