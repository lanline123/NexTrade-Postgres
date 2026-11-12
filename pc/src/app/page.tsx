import HomeClient from '@/components/HomeClient';
import { MIGRATION_ROUTES } from '@/lib/routes';

// 真正走 SSR：每次请求在服务端渲染，不做构建期静态化。
export const dynamic = 'force-dynamic';

export const metadata = {
    title: 'NexTrade PC · 迁移看板'
};

export default function Page() {
    const renderedAt = new Date().toISOString();
    const done = MIGRATION_ROUTES.filter((route) => route.status === 'done').length;

    return <HomeClient routes={MIGRATION_ROUTES} done={done} renderedAt={renderedAt} />;
}
