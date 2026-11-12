'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Empty, Pagination } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';
import { formatPrice, productImage } from '@/lib/format';
import type { CatalogQuery } from '@/lib/catalog';

/**
 * 商品结果网格（列表 / 搜索 / 店铺内 / 图搜共用）。
 * Vue 基准：pages/list/index.vue、pages/search/components/{search,CatGoodsList}.vue。
 *
 * 接口 search/search/getProduct（POST）。**分类字段是 `cat` 不是 `categoryId`**（详见 lib/catalog.ts）。
 * 两种用法：
 * - 传 `query`（配合 FilterBar，筛选/排序全由外部控制）；
 * - 传 `keyword` / `categoryId` / `shopId` 等快捷 props（内部转成同样的请求体）。
 */
interface ProductRow {
    productId?: number;
    id?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    productPrice?: number | string;
    price?: number | string;
    shopTitle?: string;
}

type ProductResultsProps = {
    query?: CatalogQuery;
    keyword?: string;
    categoryId?: number;
    shopId?: number;
    /** 图搜会话编号：带上它时结果限定在这次图片召回的商品内 */
    searchToken?: string;
    emptyText?: string;
    pageSize?: number;
    onTotal?: (total: number) => void;
    onLoading?: (loading: boolean) => void;
};

export default function ProductResults({
    query,
    keyword = '',
    categoryId,
    shopId,
    searchToken,
    emptyText,
    pageSize = 20,
    onTotal,
    onLoading,
}: ProductResultsProps) {
    const { t } = useTranslation();
    const [rows, setRows] = useState<ProductRow[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    const signature = useMemo(
        () =>
            JSON.stringify({
                query,
                keyword,
                categoryId,
                shopId,
                searchToken,
                pageSize,
            }),
        [query, keyword, categoryId, shopId, searchToken, pageSize],
    );

    // 条件变化（含筛选）回到第一页
    useEffect(() => {
        setPage(1);
    }, [signature]);

    const load = useCallback(async () => {
        setLoading(true);
        onLoading?.(true);
        try {
            const body = {
                page,
                size: pageSize,
                keyword: query?.keyword ?? (keyword || undefined),
                cat: query?.cat ?? (categoryId ? String(categoryId) : undefined),
                brand: query?.brand || undefined,
                min: query?.min || undefined,
                max: query?.max || undefined,
                sort: query?.sort || undefined,
                order: query?.order || undefined,
                attrs: query?.attrs?.length ? query.attrs : undefined,
                shopId: query?.shopId ?? shopId,
                shopCategoryId: query?.shopCategoryId || undefined,
                searchToken: query?.searchToken ?? searchToken,
            };
            const result = await apiPost<Record<string, unknown>>('search/search/getProduct', body);
            const list = (result.records ?? result.list ?? result.data ?? []) as ProductRow[];
            const rowsArray = Array.isArray(list) ? list : [];
            const nextTotal = Number(result.total ?? rowsArray.length);
            setRows(rowsArray);
            setTotal(nextTotal);
            onTotal?.(nextTotal);
            setError('');
        } catch (e) {
            setRows([]);
            setTotal(0);
            onTotal?.(0);
            setError((e as Error).message);
        } finally {
            setLoading(false);
            onLoading?.(false);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [page, pageSize, signature]);

    useEffect(() => {
        void load();
    }, [load]);

    if (loading && rows.length === 0) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('商品加载失败')} description={error} />;
    if (rows.length === 0) return <Empty title={emptyText ?? t('暂无商品')} />;

    return (
        <>
            <div className="pc-product-grid">
                {rows.map((row, index) => (
                    <a
                        className="pc-product-card"
                        key={row.productId ?? row.id ?? index}
                        href={`/product/${row.productSn ?? row.productId ?? row.id ?? ''}`}
                    >
                        <img src={productImage(row)} alt={row.productName ?? ''} />
                        <div className="pc-product-name">{row.productName}</div>
                        <div className="pc-product-price">{formatPrice(row.productPrice ?? row.price)}</div>
                        {row.shopTitle ? <div className="pc-muted">{row.shopTitle}</div> : null}
                    </a>
                ))}
            </div>
            {total > pageSize ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={pageSize} total={total} onPageChange={setPage} />
                </div>
            ) : null}
        </>
    );
}
