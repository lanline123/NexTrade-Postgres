'use client';

import { Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/member/feedback/List.vue、AskList.vue、Info.vue。 */
interface Row {
    typeName?: string;
    content?: string;
    replyContent?: string;
    addTime?: string | number;
    createTime?: string | number;
}

export default function FeedbackList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('反馈与咨询')}</Typography.Title>
            <RemoteList<Row>
                path="user/feedback/list"
                emptyText={t('暂无反馈记录')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.typeName ?? t('反馈')}</span>
                            <span className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</span>
                        </div>
                        <div className="pc-muted">{row.content ?? '-'}</div>
                        {row.replyContent ? <div className="pc-note">{t('回复')}：{row.replyContent}</div> : null}
                    </>
                )}
            />
        </div>
    );
}
