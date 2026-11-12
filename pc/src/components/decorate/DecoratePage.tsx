'use client';

import { useEffect, useState } from 'react';
import { Empty, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import DecorateModules, { type DecorateModule } from '@/components/decorate/DecorateModules';

/**
 * 装修专题页容器。
 * Vue 基准：pages/activity/index.vue —— 用 home/home/pcIndex 取 moduleList，按 type 分发。
 * 参数：?id=<装修ID> 或 ?previewId=<预览ID>。
 */
interface HomeResult {
    moduleList?: DecorateModule[];
    modules?: DecorateModule[];
    title?: string;
    description?: string;
}

export default function DecoratePage({ decorateId, previewId }: { decorateId?: number; previewId?: number }) {
    const { t } = useTranslation();
    const [data, setData] = useState<HomeResult | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    useEffect(() => {
        apiGet<HomeResult>('home/home/pcIndex', {
            id: decorateId,
            previewId
        })
            .then(setData)
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [decorateId, previewId]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('装修内容加载失败')} description={error} />;

    const modules = data?.moduleList ?? data?.modules ?? [];

    return (
        <div className="pc-catalog">
            {data?.title ? <Typography.Title heading={2}>{data.title}</Typography.Title> : null}
            {data?.description ? <Typography.Paragraph type="tertiary">{data.description}</Typography.Paragraph> : null}
            <DecorateModules modules={modules} />
        </div>
    );
}
