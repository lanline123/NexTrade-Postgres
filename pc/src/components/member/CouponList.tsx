'use client';

import { useEffect, useState } from 'react';
import { Card, Empty, Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, type Paged } from '@/lib/client-api';
import { formatDateTime, formatPrice } from '@/lib/format';

/** Vue 基准：pages/member/coupon/List.vue（我的优惠券）。 */
interface CouponRow {
    couponId?: number;
    id?: number;
    couponName?: string;
    couponMoney?: string | number;
    couponDiscount?: string | number;
    minOrderAmount?: string | number;
    useEndDate?: string | number;
    startDate?: string | number;
    status?: number;
    isUsed?: number;
}

export default function CouponList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<CouponRow[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        apiGet<Paged<CouponRow>>('user/coupon/list', { page: 1, size: 30 })
            .then((result) => setRows(result.records ?? []))
            .catch(() => setRows([]))
            .finally(() => setLoading(false));
    }, []);

    const amount = (row: CouponRow) =>
        row.couponDiscount ? `${row.couponDiscount} ${t('折')}` : formatPrice(row.couponMoney);

    return (
        <div className="pc-member-panel">
            <Typography.Title heading={4}>{t('我的优惠券')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无优惠券')} />
            ) : (
                <div className="pc-card-grid">
                    {rows.map((row, index) => (
                        <Card key={row.couponId ?? row.id ?? index} className="pc-coupon-card">
                            <div className="pc-coupon-amount">{amount(row)}</div>
                            <div className="pc-coupon-name">{row.couponName ?? t('优惠券')}</div>
                            <div className="pc-muted">
                                {t('满 {amount} 可用', { amount: formatPrice(row.minOrderAmount) })}
                            </div>
                            <div className="pc-muted">
                                {formatDateTime(row.startDate)} ~ {formatDateTime(row.useEndDate)}
                            </div>
                            <Tag size="small" color={row.isUsed || row.status === 1 ? 'grey' : 'green'}>
                                {row.isUsed || row.status === 1 ? t('已使用') : t('可使用')}
                            </Tag>
                        </Card>
                    ))}
                </div>
            )}
        </div>
    );
}
