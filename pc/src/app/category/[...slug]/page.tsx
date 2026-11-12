import type { Metadata } from 'next';
import CatalogResults from '@/components/catalog/CatalogResults';

export const dynamic = 'force-dynamic';
export const metadata: Metadata = { title: '分类 · NexTrade PC' };

/**
 * Vue 基准：pages/category/[...slug].vue —— 先按 SEO 路径解析分类 ID，再复用搜索组件。
 * 当前实现：优先取 ?cat= 数字，其次取 slug 末段数字；SEO 路径 → ID 的映射接口（getCategoryIdBySeoLink）
 * 尚未接，等分类数据确认后补。
 */
export default function Page({
    params,
    searchParams
}: {
    params: { slug?: string[] };
    searchParams?: { cat?: string };
}) {
    const fromQuery = Number(searchParams?.cat);
    const fromSlug = Number((params.slug ?? []).slice(-1)[0]);
    const cat =
        Number.isFinite(fromQuery) && fromQuery > 0
            ? String(fromQuery)
            : Number.isFinite(fromSlug) && fromSlug > 0
              ? String(fromSlug)
              : undefined;
    return <CatalogResults initialCat={cat} emptyText="该分类暂无商品" />;
}
