'use client';

import { Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice, productImage } from '@/lib/format';

/** Vue 基准：pages/member/browsingHistory/List.vue，接口 user/user/historyProduct。 */
interface Row {
    productName?: string;
    picThumb?: string;
    productPicThumb?: string;
    price?: string | number;
    productPrice?: string | number;
    addTime?: string | number;
    browseTime?: string | number;
}

export default function HistoryList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('浏览记录')}</Typography.Title>
            <RemoteList<Row>
                path="user/user/historyProduct"
                emptyText={t('暂无浏览记录')}
                renderItem={(row) => (
                    <div className="pc-media-row">
                        <img src={productImage(row)} alt={row.productName ?? ''} />
                        <div className="pc-media-main">
                            <div className="pc-ellipsis">{row.productName ?? '-'}</div>
                            <div className="pc-muted">{formatDateTime(row.addTime ?? row.browseTime)}</div>
                        </div>
                        <div className="pc-order-item-price">{formatPrice(row.price ?? row.productPrice)}</div>
                    </div>
                )}
            />
        </div>
    );
}
