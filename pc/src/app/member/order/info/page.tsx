import type { Metadata } from 'next';
import OrderDetail from '@/components/member/OrderDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = {
    title: '订单详情 · NexTrade PC',
    robots: { index: false, follow: false }
};

/** Vue 基准：pages/member/order/Info.vue；旧路由用 ?id= 传订单号 */
export default function Page({ searchParams }: { searchParams?: { id?: string } }) {
    const orderId = Number(searchParams?.id) || 0;
    return <OrderDetail orderId={orderId} />;
}
