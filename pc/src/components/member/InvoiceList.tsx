'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/userInvoice/List.vue、orderInvoice/List.vue。 */
interface Row {
    orderSn?: string;
    invoiceTypeName?: string;
    statusName?: string;
    invoiceAmount?: string | number;
    amount?: string | number;
    addTime?: string | number;
    createTime?: string | number;
}

export default function InvoiceList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('发票管理')}</Typography.Title>
            <RemoteList<Row>
                path="user/orderInvoice/list"
                emptyText={t('暂无发票')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{t('订单号')}：{row.orderSn ?? '-'}</span>
                            <Tag size="small" color="blue">{row.statusName ?? '-'}</Tag>
                        </div>
                        <div className="pc-muted">
                            {row.invoiceTypeName ?? '-'} · {formatPrice(row.invoiceAmount ?? row.amount)}
                        </div>
                        <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                    </>
                )}
            />
        </div>
    );
}
