import type { Metadata } from 'next';
import ArticleDetail from '@/components/catalog/ArticleDetail';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '帮助中心 · NexTrade PC' };

/** Vue 基准：pages/article/issue/Info.vue */
export default function Page({ searchParams }: { searchParams?: { articleId?: string; id?: string } }) {
    const articleId = Number(searchParams?.articleId ?? searchParams?.id) || 0;
    return <ArticleDetail articleId={articleId} />;
}
