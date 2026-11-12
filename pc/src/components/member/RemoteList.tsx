'use client';

import { useEffect, useState, type ReactNode } from 'react';
import { Card, Empty, Pagination } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, type Paged } from '@/lib/client-api';

/**
 * 会员中心同构列表：分页 + 登录守卫 + 空态 + 错误态。
 * 各页只提供接口路径和单行渲染，避免九份重复样板。
 */
type RemoteListProps<T> = {
    path: string;
    params?: Record<string, unknown>;
    pageSize?: number;
    emptyText?: string;
    renderItem: (item: T, index: number) => ReactNode;
};

export default function RemoteList<T>({ path, params, pageSize = 20, emptyText, renderItem }: RemoteListProps<T>) {
    const { t } = useTranslation();
    const [rows, setRows] = useState<T[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [needLogin, setNeedLogin] = useState(false);
    const paramsKey = JSON.stringify(params ?? {});

    useEffect(() => {
        const token = typeof window !== 'undefined' ? localStorage.getItem('accessToken') || localStorage.getItem('token') : null;
        if (!token) {
            setNeedLogin(true);
            setLoading(false);
            return;
        }
        setLoading(true);
        setError('');
        apiGet<Paged<T>>(path, { page, size: pageSize, ...(JSON.parse(paramsKey) as Record<string, unknown>) })
            .then((result) => {
                setRows(result.records ?? []);
                setTotal(result.total ?? 0);
            })
            .catch((e: Error) => {
                setRows([]);
                setTotal(0);
                setError(e.message);
            })
            .finally(() => setLoading(false));
    }, [path, page, pageSize, paramsKey]);

    if (needLogin) {
        return <Empty title={t('请先登录')} description={<a href="/member/login">{t('去登录')}</a>} />;
    }
    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('加载失败')} description={error} />;
    if (rows.length === 0) return <Empty title={emptyText ?? t('暂无数据')} />;

    return (
        <>
            <div className="pc-remote-list">
                {rows.map((item, index) => (
                    <Card key={index} className="pc-order-card">
                        {renderItem(item, index)}
                    </Card>
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
