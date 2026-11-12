import type { Metadata } from 'next';
import OrderList from '@/components/member/OrderList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = {
    title: '我的订单 · NexTrade PC',
    robots: { index: false, follow: false }
};

/** Vue 基准：pages/member/order/List.vue → components/member/order/MemberOrderList.vue（672 行） */
export default function Page({ searchParams }: { searchParams?: { type?: string } }) {
    // 旧版 ?type=awaitPay 等初始化 Tab
    return <OrderList initialType={searchParams?.type} />;
}
