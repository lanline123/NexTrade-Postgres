import HomePage from '@/components/home/HomePage';

// 真正走 SSR：每次请求在服务端渲染，不做构建期静态化。
export const dynamic = 'force-dynamic';

export const metadata = {
    title: 'NexTrade PC'
};

/** Vue 基准：pages/index.vue（商城首页，装修模块驱动）。迁移看板已挪到 /migration。 */
export default function Page() {
    return <HomePage />;
}
