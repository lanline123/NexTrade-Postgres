import type { Metadata } from 'next';
import CouponCenter from '@/components/catalog/CouponCenter';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '领券中心 · NexTrade PC' };

/** Vue 基准：pages/coupon/index.vue、List.vue */
export default function Page() {
    return <CouponCenter />;
}
