'use client';

import { useCallback, useEffect, useState } from 'react';
import { Card, Empty, Pagination, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, type Paged } from '@/lib/client-api';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/account/Detail.vue（余额卡 + 账户流水）。 */
interface BalanceInfo {
    balance?: number | string;
    totalBalance?: number | string;
    frozenBalance?: number | string;
}

interface FlowRow {
    logId?: number;
    changeTime?: string | number;
    changeDesc?: string;
    changeTypeName?: string;
    beforeBalance?: number | string;
    afterBalance?: number | string;
}

const PAGE_SIZE = 10;

export default function AccountDetail() {
    const { t } = useTranslation();
    const [balance, setBalance] = useState<BalanceInfo>({});
    const [rows, setRows] = useState<FlowRow[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        // 实测：user/user/detail 直接返回用户对象，没有 item 包裹
        apiGet<BalanceInfo>('user/user/detail')
            .then((result) => setBalance(result ?? {}))
            .catch(() => setBalance({}));
    }, []);

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiGet<Paged<FlowRow>>('user/account/list', { page, size: PAGE_SIZE });
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

    const delta = (row: FlowRow) => Number(row.afterBalance ?? 0) - Number(row.beforeBalance ?? 0);

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('账户余额')}</Typography.Title>

            <Card className="pc-balance-card">
                <div className="pc-balance-main">
                    <div className="pc-muted">{t('我的可用余额')}</div>
                    <div className="pc-balance-value">{formatPrice(balance.balance)}</div>
                </div>
                <div className="pc-balance-grid">
                    <div>
                        <span className="pc-muted">{t('全部余额')}</span>
                        <div>{formatPrice(balance.totalBalance)}</div>
                    </div>
                    <div>
                        <span className="pc-muted">{t('冻结余额')}</span>
                        <div>{formatPrice(balance.frozenBalance)}</div>
                    </div>
                </div>
            </Card>

            <Card title={t('账户流水')}>
                {loading ? (
                    <div className="pc-loading">{t('加载中')}</div>
                ) : rows.length === 0 ? (
                    <Empty title={t('暂无流水')} />
                ) : (
                    rows.map((row, index) => (
                        <div className="pc-flow-row" key={row.logId ?? index}>
                            <div className="pc-flow-main">
                                <div>{row.changeDesc ?? row.changeTypeName ?? t('账户变动')}</div>
                                <div className="pc-muted">{formatDateTime(row.changeTime)}</div>
                            </div>
                            <div className={delta(row) >= 0 ? 'pc-point-plus' : 'pc-point-minus'}>
                                {delta(row) >= 0 ? '+' : ''}
                                {formatPrice(Math.abs(delta(row)))}
                            </div>
                            <div className="pc-muted">{t('余额')} {formatPrice(row.afterBalance)}</div>
                        </div>
                    ))
                )}
            </Card>

            {total > PAGE_SIZE ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={PAGE_SIZE} total={total} onPageChange={setPage} />
                </div>
            ) : null}
        </div>
    );
}
