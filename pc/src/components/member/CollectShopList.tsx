'use client';

import { Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime } from '@/lib/format';

/** Vue 基准：pages/member/collectShop/List.vue。 */
interface Row {
    shopName?: string;
    storeName?: string;
    shopLogo?: string;
    logo?: string;
    addTime?: string | number;
    createTime?: string | number;
}

export default function CollectShopList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('收藏店铺')}</Typography.Title>
            <RemoteList<Row>
                path="user/user/collectionShop"
                emptyText={t('暂无收藏店铺')}
                renderItem={(row) => (
                    <div className="pc-media-row">
                        <img src={row.shopLogo ?? row.logo ?? '/assets/avatar/default.gif'} alt={row.shopName ?? ''} />
                        <div className="pc-media-main">
                            <div className="pc-ellipsis">{row.shopName ?? row.storeName ?? '-'}</div>
                            <div className="pc-muted">{formatDateTime(row.addTime ?? row.createTime)}</div>
                        </div>
                    </div>
                )}
            />
        </div>
    );
}
