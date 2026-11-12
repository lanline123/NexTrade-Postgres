import type { Metadata } from 'next';
import ArticleList from '@/components/catalog/ArticleList';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '帮助中心 · NexTrade PC' };

/** Vue 基准：pages/article/issue/Index.vue（左侧栏目树未迁，先复用文章列表流） */
export default function Page() {
    return <ArticleList />;
}
