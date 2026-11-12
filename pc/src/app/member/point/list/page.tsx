import type { Metadata } from 'next';
import PointsList from '@/components/member/PointsList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = {
    title: '我的积分 · NexTrade PC',
    robots: { index: false, follow: false }
};

/** Vue 基准：pages/member/point/List.vue；积分名未配置时组件内默认「积分」 */
export default function Page() {
    return <PointsList />;
}
