'use client';

import { useEffect, useState } from 'react';
import { Empty, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatDateTime } from '@/lib/format';

/**
 * 文章 / 公告详情。
 * Vue 基准：pages/article/news/Info.vue、issue/Info.vue。
 * 接口：article/article/{id}（Vue 侧 url 为 "article/article/" + id）。
 */
interface ArticleData {
    articleId?: number;
    title?: string;
    articleTitle?: string;
    content?: string;
    articleContent?: string;
    addTime?: string | number;
    publishTime?: string | number;
}

export default function ArticleDetail({ articleId }: { articleId: number }) {
    const { t } = useTranslation();
    const [data, setData] = useState<ArticleData | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    useEffect(() => {
        if (!articleId) {
            setError(t('缺少文章参数'));
            setLoading(false);
            return;
        }
        apiGet<ArticleData>(`article/article/${articleId}`)
            .then(setData)
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [articleId, t]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('文章加载失败')} description={error} />;
    if (!data) return <Empty title={t('文章不存在')} />;

    const html = data.content ?? data.articleContent ?? '';

    return (
        <article className="pc-article-detail">
            <Typography.Title heading={2}>{data.title ?? data.articleTitle}</Typography.Title>
            <div className="pc-muted">{formatDateTime(data.addTime ?? data.publishTime)}</div>
            {/* 富文本来自自家后端文章接口；如需支持第三方投稿，应在此接 DOMPurify 白名单 */}
            <div className="pc-article-body" dangerouslySetInnerHTML={{ __html: html }} />
        </article>
    );
}
