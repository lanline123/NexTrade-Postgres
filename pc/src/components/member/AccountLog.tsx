'use client';

import { Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/account/Log.vue（充值/提现申请记录）。 */
interface Row {
    addTime?: string | number;
    amount?: string | number;
    type?: string;
    postscript?: string;
    statusType?: string;
}

export default function AccountLog() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('申请记录')}</Typography.Title>
            <RemoteList<Row>
                path="user/rechargeOrder/list"
                emptyText={t('暂无申请记录')}
                renderItem={(row) => (
                    <div className="pc-log-row">
                        <span>{formatDateTime(row.addTime)}</span>
                        <span>{formatPrice(row.amount)}</span>
                        <span>{row.type ?? '-'}</span>
                        <span className="pc-muted">{row.postscript || '-'}</span>
                        <span>{row.statusType ?? '-'}</span>
                    </div>
                )}
            />
        </div>
    );
}
