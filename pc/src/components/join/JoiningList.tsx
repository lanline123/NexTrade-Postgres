'use client';

import { useEffect, useState } from 'react';
import { Card, Empty, Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/joining/List.vue；接口 merchant/merchant/myApply。 */
interface ApplyRow {
    id?: number;
    status?: number;
    statusName?: string;
    shopName?: string;
    merchantName?: string;
    addTime?: string | number;
    createTime?: string | number;
    remark?: string;
}

const STATUS_TEXT: Record<number, string> = {
    0: '待审核',
    1: '审核中',
    2: '已通过',
    3: '已驳回'
};

export default function JoiningList() {
    const { t } = useTranslation();
    const [apply, setApply] = useState<ApplyRow | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        apiGet<ApplyRow | ApplyRow[]>('merchant/merchant/myApply')
            .then((result) => {
                if (Array.isArray(result)) setApply(result[0] ?? null);
                else setApply(result && (result.id || result.status !== undefined) ? result : null);
            })
            .catch(() => setApply(null))
            .finally(() => setLoading(false));
    }, []);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (!apply) return <Empty title={t('暂无入驻申请')} description={<a href="/join">{t('去申请入驻')}</a>} />;

    return (
        <div className="pc-catalog">
            <Typography.Title heading={3}>{t('入驻申请')}</Typography.Title>
            <Card>
                <div className="pc-row-head">
                    <span>{apply.shopName ?? apply.merchantName ?? t('入驻申请')}</span>
                    <Tag size="small" color={apply.status === 2 ? 'green' : apply.status === 3 ? 'red' : 'blue'}>
                        {apply.statusName ?? t(STATUS_TEXT[apply.status ?? 0] ?? '待审核')}
                    </Tag>
                </div>
                <div className="pc-muted">{formatDateTime(apply.addTime ?? apply.createTime)}</div>
                {apply.remark ? <div className="pc-muted">{apply.remark}</div> : null}
            </Card>
        </div>
    );
}
