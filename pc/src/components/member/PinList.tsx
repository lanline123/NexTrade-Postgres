'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/member/pin/List.vue、Group.vue；接口 promotion/groupon/record/list。 */
interface Row {
    productName?: string;
    statusName?: string;
    grouponNum?: number;
    peopleNum?: number;
    addTime?: string | number;
    createTime?: string | number;
}

export default function PinList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('我的团购')}</Typography.Title>
            <RemoteList<Row>
                path="promotion/groupon/record/list"
                emptyText={t('暂无团购记录')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.productName ?? '-'}</span>
                            <Tag size="small" color="blue">{row.statusName ?? '-'}</Tag>
                        </div>
                        <div className="pc-muted">
                            {t('成团人数')} {row.grouponNum ?? row.peopleNum ?? '-'} · {formatDateTime(row.addTime ?? row.createTime)}
                        </div>
                    </>
                )}
            />
        </div>
    );
}
