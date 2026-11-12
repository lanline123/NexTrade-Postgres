'use client';

import { useEffect, useState } from 'react';
import { Empty } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import DecorateModules, { type DecorateModule } from '@/components/decorate/DecorateModules';

/**
 * 商城首页。
 * Vue 基准：pages/index.vue —— 取 home/home/pcIndex 的 moduleList，按 type 渲染装修模块。
 * 实测该接口返回 20 个模块（pc-home-quick-nav / pc-home-hero / pc-banner2 / pc-product …），
 * type 为 kebab-case，渲染器已统一 normalize。
 */
interface HomeResult {
    moduleList?: DecorateModule[];
    decorateId?: number;
}

export default function HomePage() {
    const { t } = useTranslation();
    const [modules, setModules] = useState<DecorateModule[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    useEffect(() => {
        apiGet<HomeResult>('home/home/pcIndex')
            .then((result) => setModules(result?.moduleList ?? []))
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, []);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('首页加载失败')} description={error} />;
    if (modules.length === 0) return <Empty title={t('首页暂无装修内容')} description={<a href="/list">{t('去逛商品')}</a>} />;

    return (
        <div className="pc-home">
            <DecorateModules modules={modules} />
        </div>
    );
}
