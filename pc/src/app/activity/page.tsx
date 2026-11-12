import type { Metadata } from 'next';
import DecoratePage from '@/components/decorate/DecoratePage';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '活动专题 · NexTrade PC' };

/**
 * Vue 基准：pages/activity/index.vue（装修模块容器）。
 * 支持 ?id=<装修ID> 与 ?previewId=<预览ID>；模块渲染见 components/decorate/DecorateModules。
 */
export default function Page({ searchParams }: { searchParams?: { id?: string; previewId?: string } }) {
    const decorateId = Number(searchParams?.id) || undefined;
    const previewId = Number(searchParams?.previewId) || undefined;
    return <DecoratePage decorateId={decorateId} previewId={previewId} />;
}
