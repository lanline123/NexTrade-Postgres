import type { Metadata } from 'next';
import SeckillList from '@/components/catalog/SeckillList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '秒杀 · NexTrade PC' };

/** Vue 基准：pages/seckill/List.vue */
export default function Page() {
    return <SeckillList />;
}
