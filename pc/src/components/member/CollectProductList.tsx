'use client';

import { Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, formatPrice, productImage } from '@/lib/format';

/** Vue 基准：pages/member/collectProduct/List.vue。 */
interface Row {
    productName?: string;
    productSn?: string;
    picThumb?: string;
    productPicThumb?: string;
    price?: string | number;
    productPrice?: string | number;
    addTime?: string | number;
    createTime?: string | number;
}

export default function CollectProductList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('收藏商品')}</Typography.Title>
            <RemoteList<Row>
                path="user/collectProduct/list"
                emptyText={t('暂无收藏商品')}
                renderItem={(row) => (
                    <div className="pc-media-row">
                        <img src={productImage(row)} alt={row.productName ?? ''} />
                        <div className="pc-media-main">
                            <div className="pc-ellipsis">{row.productName ?? '-'}</div>
                            <div className="pc-muted">{row.productSn ?? ''}</div>
                            <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                        </div>
                        <div className="pc-order-item-price">{formatPrice(row.price ?? row.productPrice)}</div>
                    </div>
                )}
            />
        </div>
    );
}
