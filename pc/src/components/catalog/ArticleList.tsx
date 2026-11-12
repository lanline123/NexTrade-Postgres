'use client';

import { useCallback, useEffect, useState } from 'react';
import { Card, Empty, Pagination, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/article/news/List.vue、issue/Index.vue；接口 article/article/list。 */
interface Row {
    articleId?: number;
    title?: string;
    articleTitle?: string;
    /** 实测字段：description（摘要） */
    description?: string;
    summary?: string;
    addTime?: string | number;
    publishTime?: string | number;
}

const PAGE_SIZE = 10;

export default function ArticleList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<Row[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    const load = useCallback(async () => {
        setLoading(true);
        try {
            const result = await apiGet<Record<string, unknown>>('article/article/list', { page, size: PAGE_SIZE });
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
            <Typography.Title heading={3}>{t('文章与公告')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无文章')} />
            ) : (
                rows.map((row, index) => (
                    <Card key={row.articleId ?? index} className="pc-article-card">
                        <div className="pc-article-title">{row.title ?? row.articleTitle ?? '-'}</div>
                        {(row.description ?? row.summary) ? (
                            <div className="pc-muted">{row.description ?? row.summary}</div>
                        ) : null}
                        <div className="pc-muted">{formatDateTime(row.addTime ?? row.publishTime)}</div>
                    </Card>
                ))
            )}
            {total > PAGE_SIZE ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={PAGE_SIZE} total={total} onPageChange={setPage} />
                </div>
            ) : null}
        </div>
    );
}
