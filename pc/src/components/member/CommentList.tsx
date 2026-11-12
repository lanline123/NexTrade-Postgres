'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/member/comment/List.vue、Info.vue。 */
interface Row {
    productName?: string;
    content?: string;
    commentContent?: string;
    score?: number;
    commentScore?: number;
    addTime?: string | number;
    createTime?: string | number;
}

export default function CommentList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('我的评价')}</Typography.Title>
            <RemoteList<Row>
                path="user/comment/list"
                emptyText={t('暂无评价')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.productName ?? '-'}</span>
                            <Tag size="small" color="amber">{row.score ?? row.commentScore ?? '-'} {t('分')}</Tag>
                        </div>
                        <div className="pc-muted">{row.content ?? row.commentContent ?? '-'}</div>
                        <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                    </>
                )}
            />
        </div>
    );
}
