'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/inquiry/List.vue、Info.vue。 */
interface Row {
    productName?: string;
    quantity?: number;
    price?: string | number;
    statusName?: string;
    addTime?: string | number;
    createTime?: string | number;
}

export default function InquiryList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('询价单')}</Typography.Title>
            <RemoteList<Row>
                path="user/priceInquiry/list"
                emptyText={t('暂无询价单')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>{row.productName ?? '-'}</span>
                            <Tag size="small" color="blue">{row.statusName ?? '-'}</Tag>
                        </div>
                        <div className="pc-muted">
                            {t('数量')} {row.quantity ?? '-'} · {formatPrice(row.price)}
                        </div>
                        <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                    </>
                )}
            />
        </div>
    );
}
