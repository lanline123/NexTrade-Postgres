'use client';

import { Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import RemoteList from '@/components/member/RemoteList';
import { formatDateTime, productImage } from '@/lib/format';

/**
 * 售后退款。
 * Vue 基准：pages/member/return/List.vue。
 * 实测 user/aftersales/list 返回的是「可申请售后的订单」（orderId/orderSn/shippingTime/items），
 * 不是售后单列表，所以这里按订单维度展示，售后单进度从订单详情进入。
 */
interface Row {
    orderId?: number;
    orderSn?: string;
    shippingTime?: string | number;
    items?: Array<{ productName?: string; picThumb?: string; productImage?: string; quantity?: number }>;
}

export default function AfterSalesList() {
    const { t } = useTranslation();
    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('售后退款')}</Typography.Title>
            <div className="pc-muted">{t('以下订单可申请售后；售后单进度请从订单详情进入。')}</div>
            <RemoteList<Row>
                path="user/aftersales/list"
                emptyText={t('暂无可申请售后的订单')}
                renderItem={(row) => (
                    <>
                        <div className="pc-row-head">
                            <span>
                                {t('订单号')}：{row.orderSn ?? '-'}
                            </span>
                            <Tag size="small" color="blue">{t('可申请售后')}</Tag>
                        </div>
                        {(row.items ?? []).map((item, index) => (
                            <div className="pc-media-row" key={index}>
                                <img src={productImage(item)} alt={item.productName ?? ''} />
                                <div className="pc-media-main">
                                    <div className="pc-ellipsis">{item.productName}</div>
                                    <div className="pc-muted">{t('数量')} {item.quantity ?? '-'}</div>
                                </div>
                            </div>
                        ))}
                        <div className="pc-muted">
                            {t('发货时间')} {formatDateTime(row.shippingTime) || '-'}
                        </div>
                    </>
                )}
            />
        </div>
    );
}
