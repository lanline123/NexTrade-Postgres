'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Avatar, Card, Empty, Tag, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet } from '@/lib/client-api';
import { formatPrice } from '@/lib/format';
import { MEMBER_MENU } from '@/lib/member-menu';

/** Vue 基准：pages/member/index.vue（用户信息 + 订单角标 + 快捷入口）。 */
interface CenterData {
    nickname?: string;
    avatar?: string;
    rankName?: string;
    balance?: string | number;
    points?: number;
    couponCount?: number;
}

export default function MemberHome() {
    const { t } = useTranslation();
    const [center, setCenter] = useState<CenterData | null>(null);
    const [orderNum, setOrderNum] = useState<Record<string, number>>({});
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');

    useEffect(() => {
        Promise.all([
            apiGet<CenterData>('user/user/memberCenter').catch(() => null),
            apiGet<Record<string, number>>('user/order/orderNum').catch(() => ({}))
        ])
            .then(([info, counts]) => {
                setCenter(info);
                setOrderNum(counts ?? {});
                if (!info) setError(t('登录后查看账户信息'));
            })
            .catch((e: Error) => setError(e.message))
            .finally(() => setLoading(false));
    }, [t]);

    const stats = [
        { label: t('待付款'), value: orderNum.awaitPay ?? '-', href: '/member/order/list?type=awaitPay' },
        { label: t('待发货'), value: orderNum.awaitShipping ?? '-', href: '/member/order/list?type=awaitShipping' },
        { label: t('待收货'), value: orderNum.awaitReceived ?? '-', href: '/member/order/list?type=awaitReceived' },
        { label: t('待评价'), value: orderNum.awaitComment ?? '-', href: '/member/order/list?type=awaitComment' }
    ];

    return (
        <div className="pc-member-panel">
            <Card className="pc-home-profile">
                <Avatar size="large" src={center?.avatar || '/assets/avatar/default.gif'} alt={center?.nickname ?? ''} />
                <div className="pc-home-profile-info">
                    <Typography.Title heading={4}>{center?.nickname ?? t('会员中心')}</Typography.Title>
                    <div className="pc-muted">
                        {center?.rankName ? <Tag size="small" color="amber">{center.rankName}</Tag> : null}
                        {t('账户余额')} {formatPrice(center?.balance)} · {t('积分')} {center?.points ?? '-'}
                    </div>
                </div>
            </Card>

            {error ? <div className="pc-note">{error}</div> : null}

            <div className="pc-stat-grid">
                {stats.map((item) => (
                    <Link key={item.label} href={item.href} className="pc-stat-card">
                        <div className="pc-stat-value">{loading ? '-' : item.value}</div>
                        <div className="pc-muted">{item.label}</div>
                    </Link>
                ))}
            </div>

            <Card title={t('快捷入口')}>
                <div className="pc-overview-links">
                    {MEMBER_MENU.flatMap((group) => group.items)
                        .slice(0, 8)
                        .map((item) => (
                            <Link key={item.path} href={item.path} className="pc-overview-link">
                                {t(item.label)}
                            </Link>
                        ))}
                </div>
            </Card>

            {!loading && !center && !error ? <Empty title={t('暂无数据')} /> : null}
        </div>
    );
}
