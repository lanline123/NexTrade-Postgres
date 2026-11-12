import type { Metadata } from 'next';
import GrouponList from '@/components/catalog/GrouponList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '拼团 · NexTrade PC' };

/** Vue 基准：pages/joining/List.vue + components/common/JoiningGoodsList.vue（拼团商品列表） */
export default function Page() {
    return <GrouponList />;
}
