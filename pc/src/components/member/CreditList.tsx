'use client';

import { Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/termPay/list.vue；接口 user/user/creditDetail。 */
interface Row {
    amount?: string | number;
    type?: string;
    typeName?: string;
    addTime?: string | number;
    createTime?: string | number;
    remark?: string;
}

export default function CreditList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('账期明细')}</Typography.Title>
            <RemoteList<Row>
                path="user/user/creditDetail"
                emptyText={t('暂无账期记录')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.typeName ?? row.type ?? t('账期变动')}</span>
                            <span>{formatPrice(row.amount)}</span>
                        </div>
                        <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                        {row.remark ? <div className="pc-muted">{row.remark}</div> : null}
                    </>
                )}
            />
        </div>
    );
}
