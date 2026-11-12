'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/member/userMessage/List.vue。 */
interface Row {
    messageTitle?: string;
    title?: string;
    messageContent?: string;
    content?: string;
    isRead?: number;
    addTime?: string | number;
    createTime?: string | number;
}

export default function MessageList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('我的消息')}</Typography.Title>
            <RemoteList<Row>
                path="user/message/list"
                emptyText={t('暂无消息')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.messageTitle ?? row.title ?? '-'}</span>
                            <Tag size="small" color={row.isRead ? 'grey' : 'green'}>
                                {row.isRead ? t('已读') : t('未读')}
                            </Tag>
                        </div>
                        <div className="pc-muted">{row.messageContent ?? row.content ?? '-'}</div>
                        <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                    </>
                )}
            />
        </div>
    );
}
