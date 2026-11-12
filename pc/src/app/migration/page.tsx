import HomeClient from '@/components/HomeClient';
import { MIGRATION_ROUTES } from '@/lib/routes';

export const dynamic = 'force-dynamic';

export const metadata = {
    title: '迁移看板 · NexTrade PC',
    robots: { index: false, follow: false }
};

/** 迁移看板（原挂在 /，首页接管后挪到这里）。 */
export default function Page() {
    const renderedAt = new Date().toISOString();
    const done = MIGRATION_ROUTES.filter((route) => route.status === 'done').length;

    return <HomeClient routes={MIGRATION_ROUTES} done={done} renderedAt={renderedAt} />;
}
