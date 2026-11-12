import type { Metadata } from 'next';
import CouponList from '@/components/member/CouponList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = {
    title: '我的优惠券 · NexTrade PC',
    robots: { index: false, follow: false }
};

/** Vue 基准：pages/member/coupon/List.vue */
export default function Page() {
    return <CouponList />;
}
