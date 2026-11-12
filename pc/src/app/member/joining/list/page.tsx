import type { Metadata } from 'next';
import MyGrouponList from '@/components/join/MyGrouponList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的拼团 · NexTrade PC', robots: { index: false, follow: false } };

/** Vue 基准：pages/member/joining/List.vue + components/member/joining/JoiningOrderList.vue */
export default function Page() {
    return <MyGrouponList />;
}
