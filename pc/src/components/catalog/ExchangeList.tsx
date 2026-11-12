'use client';

import { useCallback, useEffect, useState } from 'react';
import { Empty, Pagination, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatPrice, productImage } from '@/lib/format';

/** Vue 基准：pages/exchange/List.vue；接口 product/exchange/list。 */
interface Row {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    exchangePrice?: number | string;
    productPrice?: number | string;
}

const PAGE_SIZE = 20;

export default function ExchangeList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<Row[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiGet<Record<string, unknown>>('product/exchange/list', { page, size: PAGE_SIZE });
            const list = (result.records ?? result.list ?? result.data ?? []) as Row[];
            const rowsArray = Array.isArray(list) ? list : [];
            setRows(rowsArray);
            setTotal(Number(result.total ?? rowsArray.length));
        } catch {
            setRows([]);
            setTotal(0);
        } finally {
            setLoading(false);
        }
    }, [page]);

    useEffect(() => {
        void load();
    }, [load]);

    return (
        <div className="pc-catalog">
            <Typography.Title heading={3}>{t('换购专区')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无可换购商品')} />
            ) : (
                <div className="pc-product-grid">
                    {rows.map((row, index) => (
                        <a className="pc-product-card" key={row.productId ?? index} href={`/product/${row.productSn ?? row.productId ?? ''}`}>
                            <img src={productImage(row)} alt={row.productName ?? ''} />
                            <div className="pc-product-name">{row.productName}</div>
                            <div className="pc-product-price">{formatPrice(row.exchangePrice ?? row.productPrice)}</div>
                        </a>
                    ))}
                </div>
            )}
            {total > PAGE_SIZE ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={PAGE_SIZE} total={total} onPageChange={setPage} />
                </div>
            ) : null}
        </div>
    );
}
