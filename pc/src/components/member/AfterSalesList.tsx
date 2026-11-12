'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/return/List.vue、Info.vue、Log.vue。 */
interface Row {
    orderSn?: string;
    aftersalesSn?: string;
    productName?: string;
    statusName?: string;
    aftersalesStatusName?: string;
    refundAmount?: string | number;
    addTime?: string | number;
    createTime?: string | number;
}

export default function AfterSalesList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('售后退款')}</Typography.Title>
            <RemoteList<Row>
                path="user/aftersales/list"
                emptyText={t('暂无售后单')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.aftersalesSn ?? row.orderSn ?? '-'}</span>
                            <Tag size="small" color="blue">{row.statusName ?? row.aftersalesStatusName ?? '-'}</Tag>
                        </div>
                        <div className="pc-muted">{row.productName ?? '-'}</div>
                        <div className="pc-muted">
                            {t('退款金额')} {formatPrice(row.refundAmount)} · {formatDateTime(row.addTime ?? row.createTime)}
                        </div>
                    </>
                )}
            />
        </div>
    );
}
