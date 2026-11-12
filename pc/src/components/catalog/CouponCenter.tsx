'use client';

import { useEffect, useState } from 'react';
import { Button, Card, Empty, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';

/** Vue 基准：pages/coupon/index.vue、List.vue；接口 home/home/getCoupon + user/coupon/claim。 */
interface Row {
    couponId?: number;
    id?: number;
    couponName?: string;
    couponMoney?: number | string;
    couponDiscount?: number | string;
    minOrderAmount?: number | string;
    isReceived?: number;
}

export default function CouponCenter() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<Row[]>([]);
    const [loading, setLoading] = useState(true);
    const [busyId, setBusyId] = useState(0);

    const load = () => {
        setLoading(true);
        apiGet<Record<string, unknown>>('home/home/getCoupon')
            .then((result) => {
                const list = (result.records ?? result.list ?? result.data ?? []) as Row[];
                setRows(Array.isArray(list) ? list : []);
            })
            .catch(() => setRows([]))
            .finally(() => setLoading(false));
    };

    useEffect(load, []);

    const claim = async (row: Row) => {
        const id = row.couponId ?? row.id ?? 0;
        setBusyId(id);
        try {
            await apiPost('user/coupon/claim', { couponId: id, id });
            Toast.success(t('领取成功'));
            load();
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setBusyId(0);
        }
    };

    return (
        <div className="pc-catalog">
            <Typography.Title heading={3}>{t('领券中心')}</Typography.Title>
            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无可领优惠券')} />
            ) : (
                <div className="pc-card-grid">
                    {rows.map((row, index) => (
                        <Card key={row.couponId ?? row.id ?? index} className="pc-coupon-card">
                            <div className="pc-coupon-amount">
                                {row.couponDiscount ? `${row.couponDiscount} ${t('折')}` : formatPrice(row.couponMoney)}
                            </div>
                            <div className="pc-coupon-name">{row.couponName ?? t('优惠券')}</div>
                            <div className="pc-muted">{t('满 {amount} 可用', { amount: formatPrice(row.minOrderAmount) })}</div>
                            <Button
                                size="small"
                                theme="solid"
                                type="primary"
                                loading={busyId === (row.couponId ?? row.id)}
                                disabled={!!row.isReceived}
                                onClick={() => void claim(row)}
                            >
                                {row.isReceived ? t('已领取') : t('立即领取')}
                            </Button>
                        </Card>
                    ))}
                </div>
            )}
        </div>
    );
}
