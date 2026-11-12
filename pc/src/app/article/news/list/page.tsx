import type { Metadata } from 'next';
import ArticleList from '@/components/catalog/ArticleList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '文章与公告 · NexTrade PC' };

/** Vue 基准：pages/article/news/List.vue */
export default function Page() {
    return <ArticleList />;
}
