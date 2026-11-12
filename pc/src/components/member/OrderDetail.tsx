'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card, Empty, Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatDateTime, formatPrice, productImage, skuText } from '@/lib/format';

/** Vue 基准：pages/member/order/Info.vue + src/OrderProductTable.vue / OrderInfoHeader.vue。 */
interface DetailItem {
    productName?: string;
    picThumb?: string;
    productPicThumb?: string;
    quantity?: number;
    price?: string | number;
    skuData?: unknown;
}

interface OrderDetailData {
    orderId?: number;
    orderSn?: string;
    addTime?: string | number;
    orderStatusName?: string;
    totalAmount?: string | number;
    shippingFee?: string | number;
    consignee?: string;
    mobile?: string;
    address?: string;
    regionName?: string;
    items?: DetailItem[];
}

export default function OrderDetail({ orderId }: { orderId: number }) {
    const { t } = useTranslation();
    const [detail, setDetail] = useState<OrderDetailData | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    useEffect(() => {
        if (!orderId) {
            setError(t('缺少订单号'));
            setLoading(false);
            return;
        }
        apiGet<OrderDetailData>('user/order/detail', { id: orderId })
            .then(setDetail)
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [orderId, t]);

    if (loading) return <div className="pc-loading">{t('加载中')}</div>;
    if (error) return <Empty title={t('订单加载失败')} description={error} />;
    if (!detail) return <Empty title={t('暂无订单')} />;

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('订单详情')}</Typography.Title>
                <Link href="/member/order/list">
                    <Button theme="light" size="small">{t('返回列表')}</Button>
                </Link>
            </div>

            <Card className="pc-order-card">
                <div className="pc-order-head">
                    <span>{t('订单号')}：{detail.orderSn ?? detail.orderId}</span>
                    <span>{formatDateTime(detail.addTime)}</span>
                    <Tag color="blue" size="small">{detail.orderStatusName ?? t('处理中')}</Tag>
                </div>
                <div className="pc-detail-grid">
                    <div><span className="pc-muted">{t('收货人')}</span> {detail.consignee ?? '-'}</div>
                    <div><span className="pc-muted">{t('联系电话')}</span> {detail.mobile ?? '-'}</div>
                    <div><span className="pc-muted">{t('收货地址')}</span> {[detail.regionName, detail.address].filter(Boolean).join(' ') || '-'}</div>
                    <div><span className="pc-muted">{t('运费')}</span> {formatPrice(detail.shippingFee)}</div>
                </div>
            </Card>

            <Card className="pc-order-card" title={t('商品清单')}>
                {(detail.items ?? []).map((item, index) => (
                    <div className="pc-order-item" key={index}>
                        <img src={productImage(item)} alt={item.productName ?? ''} />
                        <div className="pc-order-item-main">
                            <div className="pc-ellipsis">{item.productName}</div>
                            <div className="pc-muted">{skuText(item.skuData)}</div>
                        </div>
                        <div className="pc-order-item-price">{formatPrice(item.price)} × {item.quantity ?? 1}</div>
                    </div>
                ))}
                <div className="pc-order-foot">
                    <span>{t('合计')} {formatPrice(detail.totalAmount)}</span>
                </div>
            </Card>
        </div>
    );
}
