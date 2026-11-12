'use client';

import { useCallback, useEffect, useState } from 'react';
import { Card, Empty, Pagination, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/shop/List.vue；接口 shop/shop/list。 */
interface ShopRow {
    shopId?: number;
    shopName?: string;
    shopTitle?: string;
    shopLogo?: string;
    logo?: string;
    addTime?: string | number;
    createTime?: string | number;
}

const PAGE_SIZE = 12;

export default function ShopList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<ShopRow[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiGet<Record<string, unknown>>('shop/shop/list', { page, size: PAGE_SIZE });
            const list = (result.records ?? result.list ?? result.data ?? []) as ShopRow[];
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
            <Typography.Title heading={3}>{t('店铺')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无店铺')} />
            ) : (
                <div className="pc-card-grid">
                    {rows.map((row, index) => (
                        <a className="pc-shop-card" key={row.shopId ?? index} href={`/shop/${row.shopId ?? ''}`}>
                            <img src={row.shopLogo ?? row.logo ?? '/assets/avatar/default.gif'} alt={row.shopName ?? ''} />
                            <div className="pc-shop-name">{row.shopName ?? row.shopTitle ?? '-'}</div>
                            <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
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
