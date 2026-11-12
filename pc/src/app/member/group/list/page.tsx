import type { Metadata } from 'next';
import MyGrouponList from '@/components/join/MyGrouponList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '我的拼团 · NexTrade PC', robots: { index: false, follow: false } };

/**
 * Vue 基准：pages/member/group/List.vue —— 与 /member/joining/list 渲染同一个组件
 * （Vue 里是「跨境站用 /member/group/list」的分支，内容都是 JoiningOrderList）。
 */
export default function Page() {
    return <MyGrouponList />;
}
