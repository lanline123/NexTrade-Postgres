import HomePage from '@/components/home/HomePage';
import { serverGet } from '@/lib/server-api';
import type { DecorateModule } from '@/components/decorate/DecorateModules';

// 真正走 SSR：每次请求在服务端渲染，不做构建期静态化。
export const dynamic = 'force-dynamic';

export const metadata = {
    title: 'NexTrade PC'
};

interface HomeResult {
    moduleList?: DecorateModule[];
}

/**
 * Vue 基准：pages/index.vue（商城首页，装修模块驱动）。迁移看板已挪到 /migration。
 *
 * 首页装修是公开接口（`home/home/pcIndex` 免登录），因此在服务端取好后当 props 下发：
 * 首屏 HTML 里就是完整楼层，而不是「空壳 + 浏览器再请求」。
 * 取失败时 `modules` 为 undefined，由 HomePage 用浏览器端请求兜底，不会白屏。
 */
interface PromotionResult<T> {
    records?: T[];
}

export default async function Page() {
    // 三个接口都是公开的，并行取；任一失败该部分降级为客户端请求
    const [home, seckill, groupon] = await Promise.all([
        serverGet<HomeResult>('home/home/pcIndex'),
        serverGet<PromotionResult<unknown>>('home/home/getSeckill', { page: 1, size: 5 }),
        serverGet<PromotionResult<unknown>>('promotion/groupon/itemList', { page: 1, size: 5 })
    ]);

    return (
        <HomePage
            modules={home?.moduleList ?? undefined}
            preload={{
                seckill: seckill?.records,
                groupon: groupon?.records
            }}
        />
    );
}
