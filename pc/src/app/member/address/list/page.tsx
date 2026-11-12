import type { Metadata } from 'next';
import AddressList from '@/components/member/AddressList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = {
    title: '收货地址 · NexTrade PC',
    robots: { index: false, follow: false }
};

/** Vue 基准：pages/member/address/List.vue、EditRegion.vue */
export default function Page() {
    return <AddressList />;
}
