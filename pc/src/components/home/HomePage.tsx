'use client';

import { useEffect, useState } from 'react';
import { Empty } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import DecorateModules, { type DecorateModule, type DecoratePreload } from '@/components/decorate/DecorateModules';

/**
 * 商城首页。
 * Vue 基准：pages/index.vue —— 取 home/home/pcIndex 的 moduleList，按 type 渲染装修模块。
 *
 * 数据优先由**服务端**传入（`app/page.tsx` 里 serverGet 直连后端），首屏 HTML 即带装修内容；
 * 服务端取失败（后端抖动/超时）时，这里再用浏览器端请求兜底，不让首页直接空掉。
 */
interface HomeResult {
    moduleList?: DecorateModule[];
    decorateId?: number;
}

export default function HomePage({
    modules: incoming,
    preload,
}: {
    modules?: DecorateModule[];
    preload?: DecoratePreload;
}) {
    const { t } = useTranslation();
    const [modules, setModules] = useState<DecorateModule[]>(incoming ?? []);
    const [loading, setLoading] = useState(!(incoming && incoming.length > 0));
    const [error, setError] = useState('');

    useEffect(() => {
        if (incoming && incoming.length > 0) return;
        setLoading(true);
        apiGet<HomeResult>('home/home/pcIndex')
            .then((result) => setModules(result?.moduleList ?? []))
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [incoming]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('首页加载失败')} description={error} />;
    if (modules.length === 0) return <Empty title={t('首页暂无装修内容')} description={<a href="/list">{t('去逛商品')}</a>} />;

    return (
        <div className="pc-home">
            <DecorateModules modules={modules} preload={preload} />
        </div>
    );
}
