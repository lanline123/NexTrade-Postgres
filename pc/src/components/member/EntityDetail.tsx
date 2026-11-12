'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card, Empty, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatDateTime, formatPrice } from '@/lib/format';
import { MEMBER_DETAILS, type DetailField } from '@/lib/member-details';

/**
 * 会员详情页通用壳：一处取数 + 字段表渲染，19 个详情页共用。
 * 配置在 src/lib/member-details.ts（接口、字段、多候选键）。
 */
function pick(row: Record<string, unknown>, field: DetailField): unknown {
    const keys = [field.key, ...(field.fallbacks ?? [])];
    for (const key of keys) {
        const value = row[key];
        if (value !== undefined && value !== null && value !== '') return value;
    }
    return undefined;
}

function renderValue(value: unknown, field: DetailField): string {
    if (value === undefined) return '-';
    if (field.type === 'price') return formatPrice(value);
    if (field.type === 'datetime') return formatDateTime(value as number | string) || '-';
    return String(value);
}

export default function EntityDetail({ configKey, id }: { configKey: string; id?: string }) {
    const { t } = useTranslation();
    const config = MEMBER_DETAILS[configKey];
    const [row, setRow] = useState<Record<string, unknown> | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    useEffect(() => {
        if (!config) {
            setError('未配置的详情页');
            setLoading(false);
            return;
        }
        apiGet<Record<string, unknown> | Record<string, unknown>[]>(config.api, {
            [config.idParam ?? 'id']: id
        })
            .then((result) => {
                if (Array.isArray(result)) setRow(result[0] ?? null);
                else setRow(result ?? null);
            })
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [config, id]);

    if (!config) return <Empty title={t('页面未配置')} />;
    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('详情加载失败')} description={error} />;
    if (!row) return <Empty title={t('暂无数据')} />;

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t(config.title)}</Typography.Title>
                <Link href="/member">
                    <Button theme="light" size="small">{t('返回会员中心')}</Button>
                </Link>
            </div>

            <Card>
                <div className="pc-detail-grid">
                    {config.fields.map((field) => (
                        <div key={field.key}>
                            <span className="pc-muted">{t(field.label)}</span>
                            <div>{renderValue(pick(row, field), field)}</div>
                        </div>
                    ))}
                </div>
            </Card>

            <div className="pc-muted">
                {t('原 Vue 页面')}：<code>{config.source}</code>
            </div>
        </div>
    );
}
