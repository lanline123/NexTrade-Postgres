'use client';

import { useCallback, useEffect, useState } from 'react';
import { Empty, Pagination } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';
import { formatPrice, productImage } from '@/lib/format';

/**
 * 商品结果网格（列表 / 搜索共用）。
 * Vue 基准：pages/list/index.vue、pages/search/components/{search,CatGoodsList}.vue。
 * 接口 search/search/getProduct（POST）。筛选栏（分类/品牌/属性）未迁，先把结果流打通。
 */
interface ProductRow {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    productPrice?: number | string;
    price?: number | string;
    shopTitle?: string;
}

type ProductResultsProps = {
    keyword?: string;
    categoryId?: number;
    shopId?: number;
    emptyText?: string;
    pageSize?: number;
};

export default function ProductResults({ keyword = '', categoryId, shopId, emptyText, pageSize = 20 }: ProductResultsProps) {
    const { t } = useTranslation();
    const [rows, setRows] = useState<ProductRow[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiPost<Record<string, unknown>>('search/search/getProduct', {
                page,
                size: pageSize,
                keyword: keyword || undefined,
                categoryId,
                shopId
            });
            const list = (result.records ?? result.list ?? result.data ?? []) as ProductRow[];
            const rowsArray = Array.isArray(list) ? list : [];
            setRows(rowsArray);
            setTotal(Number(result.total ?? rowsArray.length));
            setError('');
        } catch (e) {
            setRows([]);
            setTotal(0);
            setError((e as Error).message);
        } finally {
            setLoading(false);
        }
    }, [page, pageSize, keyword, categoryId, shopId]);

    useEffect(() => {
        void load();
    }, [load]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('商品加载失败')} description={error} />;
    if (rows.length === 0) return <Empty title={emptyText ?? t('暂无商品')} />;

    return (
        <>
            <div className="pc-product-grid">
                {rows.map((row, index) => (
                    <a className="pc-product-card" key={row.productId ?? index} href={`/product/${row.productSn ?? row.productId ?? ''}`}>
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
