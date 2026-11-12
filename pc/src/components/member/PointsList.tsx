'use client';

import { useCallback, useEffect, useState } from 'react';
import { Card, Empty, Pagination, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, type Paged } from '@/lib/client-api';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/member/point/List.vue，接口 user/pointsLog/list。 */
interface PointRow {
    id?: number;
    logId?: number;
    changeValue?: number;
    points?: number;
    changeDesc?: string;
    description?: string;
    addTime?: string | number;
}

const PAGE_SIZE = 20;

export default function PointsList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<PointRow[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiGet<Paged<PointRow>>('user/pointsLog/list', { page, size: PAGE_SIZE });
            setRows(result.records ?? []);
            setTotal(result.total ?? 0);
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
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('积分')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无积分记录')} />
            ) : (
                <Card>
                    {rows.map((row, index) => (
                        <div className="pc-point-row" key={row.logId ?? row.id ?? index}>
                            <div>
                                <div>{row.changeDesc ?? row.description ?? t('积分变动')}</div>
                                <div className="pc-muted">{formatDateTime(row.addTime)}</div>
                            </div>
                            <div className={Number(row.changeValue ?? row.points ?? 0) >= 0 ? 'pc-point-plus' : 'pc-point-minus'}>
                                {Number(row.changeValue ?? row.points ?? 0) >= 0 ? '+' : ''}
                                {row.changeValue ?? row.points ?? 0}
                            </div>
                        </div>
                    ))}
                </Card>
            )}
            {total > PAGE_SIZE ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={PAGE_SIZE} total={total} onPageChange={setPage} />
                </div>
            ) : null}
        </div>
    );
}
