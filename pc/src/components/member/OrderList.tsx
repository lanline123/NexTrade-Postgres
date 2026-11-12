'use client';

import { useCallback, useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card, Empty, Input, Modal, Pagination, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, apiPost, type Paged } from '@/lib/client-api';
import { formatDateTime, formatPrice, productImage, skuText } from '@/lib/format';

/**
 * 我的订单列表。
 * Vue 基准：pages/member/order/List.vue → components/member/order/MemberOrderList.vue
 * 本实现只取核心链路：状态 Tab + 角标 / 关键词搜索 / 订单卡片 / 分页 / 常用操作。
 * Vue 版里的预售定金、拼团、礼品卡、线下认款等分支按页迁移时再逐项补，不在这里堆条件。
 */
interface OrderItem {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    productPicThumb?: string;
    quantity?: number;
    price?: string | number;
    skuData?: unknown;
}

interface OrderRow {
    orderId: number;
    orderSn?: string;
    addTime?: string | number;
    orderStatus?: number;
    orderStatusName?: string;
    totalAmount?: string | number;
    cancelRemark?: string;
    items?: OrderItem[];
    availableActions?: Record<string, boolean>;
}

type TabKey = 'all' | 'awaitPay' | 'awaitShipping' | 'awaitReceived' | 'awaitComment';

const TABS: Array<{ key: TabKey; label: string; orderStatus: number; commentStatus: number }> = [
    { key: 'all', label: '全部', orderStatus: -1, commentStatus: -1 },
    { key: 'awaitPay', label: '待付款', orderStatus: 0, commentStatus: -1 },
    { key: 'awaitShipping', label: '待发货', orderStatus: 1, commentStatus: -1 },
    { key: 'awaitReceived', label: '待收货', orderStatus: 2, commentStatus: -1 },
    { key: 'awaitComment', label: '待评价', orderStatus: -1, commentStatus: 0 }
];

const ACTION_ENDPOINTS: Record<string, string> = {
    cancel: 'user/order/cancelOrder',
    confirm: 'user/order/confirmReceipt',
    del: 'user/order/delOrder',
    buyAgain: 'user/order/buyAgain'
};

const PAGE_SIZE = 10;

export default function OrderList({ initialType = 'all' }: { initialType?: string }) {
    const { t } = useTranslation();
    const [tab, setTab] = useState<TabKey>(TABS.some((item) => item.key === initialType) ? (initialType as TabKey) : 'all');
    const [keyword, setKeyword] = useState('');
    const [search, setSearch] = useState('');
    const [page, setPage] = useState(1);
    const [rows, setRows] = useState<OrderRow[]>([]);
    const [total, setTotal] = useState(0);
    const [counts, setCounts] = useState<Record<string, number>>({});
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [pending, setPending] = useState<{ action: string; label: string; order: OrderRow } | null>(null);

    const load = useCallback(async () => {
        setLoading(true);
        setError('');
        const active = TABS.find((item) => item.key === tab) ?? TABS[0];
        try {
            const result = await apiGet<Paged<OrderRow>>('user/order/list', {
                page,
                size: PAGE_SIZE,
                sortField: 'addTime',
                sortOrder: 'desc',
                keyword: search || undefined,
                payStatus: -1,
                dateType: 1,
                orderStatus: active.orderStatus,
                commentStatus: active.commentStatus
            });
            setRows(result.records ?? []);
            setTotal(result.total ?? 0);
        } catch (e) {
            setRows([]);
            setTotal(0);
            setError((e as Error).message);
        } finally {
            setLoading(false);
        }
    }, [tab, page, search]);

    useEffect(() => {
        void load();
    }, [load]);

    useEffect(() => {
        apiGet<Record<string, number>>('user/order/orderNum')
            .then(setCounts)
            .catch(() => setCounts({}));
    }, []);

    const runAction = async () => {
        if (!pending) return;
        try {
            await apiPost(ACTION_ENDPOINTS[pending.action], { id: pending.order.orderId, orderId: pending.order.orderId });
            Toast.success(t('操作成功'));
            setPending(null);
            await load();
        } catch (e) {
            Toast.error((e as Error).message);
        }
    };

    const switchTab = (key: TabKey) => {
        setTab(key);
        setPage(1);
    };

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('我的订单')}</Typography.Title>
                <div className="pc-search">
                    <Input
                        value={keyword}
                        onChange={(value) => setKeyword(String(value))}
                        placeholder={t('收货人名称或订单号')}
                        style={{ width: 260 }}
                    />
                    <Button
                        theme="solid"
                        type="primary"
                        onClick={() => {
                            setSearch(keyword);
                            setPage(1);
                        }}
                    >
                        {t('搜索')}
                    </Button>
                </div>
            </div>

            <div className="pc-tabs">
                {TABS.map((item) => (
                    <button key={item.key} className={tab === item.key ? 'is-active' : ''} onClick={() => switchTab(item.key)}>
                        {t(item.label)}
                        {counts[item.key] ? ` (${counts[item.key]})` : ''}
                    </button>
                ))}
            </div>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : error ? (
                <Empty title={t('订单加载失败')} description={error} />
            ) : rows.length === 0 ? (
                <Empty title={t('暂无订单')} description={t('换个筛选条件试试')} />
            ) : (
                <div className="pc-order-list">
                    {rows.map((order) => {
                        const actions = order.availableActions ?? {};
                        const quantity = (order.items ?? []).reduce((sum, item) => sum + Number(item.quantity ?? 0), 0);
                        // 实测 availableActions 的键：cancelOrder / confirmReceipt / delOrder / rebuy
                        const buttons = [
                            { key: 'cancel', label: t('取消订单'), show: actions.cancelOrder ?? order.orderStatus === 0 },
                            { key: 'confirm', label: t('确认收货'), show: actions.confirmReceipt ?? order.orderStatus === 2 },
                            { key: 'buyAgain', label: t('再次购买'), show: actions.rebuy ?? true },
                            { key: 'del', label: t('删除订单'), show: actions.delOrder ?? order.orderStatus === 5 }
                        ].filter((item) => item.show);

                        return (
                            <Card key={order.orderId} className="pc-order-card">
                                <div className="pc-order-head">
                                    <span>
                                        {t('订单号')}：{order.orderSn ?? order.orderId}
                                    </span>
                                    <span>{formatDateTime(order.addTime)}</span>
                                    <Tag color="blue" size="small">
                                        {order.orderStatusName ?? t('处理中')}
                                    </Tag>
                                    <Link href={`/member/order/info?id=${order.orderId}`}>{t('订单详情')}</Link>
                                </div>

                                {(order.items ?? []).map((item, index) => (
                                    <div className="pc-order-item" key={`${item.productId ?? index}-${index}`}>
                                        {/* 商品图是后端返回的 URL，用原生 img 避免 next/image 的域名白名单限制 */}
                                        <img src={productImage(item)} alt={item.productName ?? ''} />
                                        <div className="pc-order-item-main">
                                            <div className="pc-ellipsis">{item.productName}</div>
                                            <div className="pc-muted">{skuText(item.skuData)}</div>
                                        </div>
                                        <div className="pc-order-item-price">
                                            {formatPrice(item.price)} × {item.quantity ?? 1}
                                        </div>
                                    </div>
                                ))}

                                {order.cancelRemark ? <div className="pc-muted">{t('取消原因')}：{order.cancelRemark}</div> : null}

                                <div className="pc-order-foot">
                                    <span>
                                        {t('共 {count} 件', { count: quantity })} · {t('合计')} {formatPrice(order.totalAmount)}
                                    </span>
                                    <div className="pc-order-actions">
                                        {buttons.map((item) => (
                                            <Button
                                                key={item.key}
                                                size="small"
                                                theme="light"
                                                onClick={() => setPending({ action: item.key, label: item.label, order })}
                                            >
                                                {item.label}
                                            </Button>
                                        ))}
                                    </div>
                                </div>
                            </Card>
                        );
                    })}
                </div>
            )}

            {total > PAGE_SIZE ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={PAGE_SIZE} total={total} onPageChange={(next) => setPage(next)} />
                </div>
            ) : null}

            <Modal
                visible={!!pending}
                title={t('确认操作')}
                onOk={() => void runAction()}
                onCancel={() => setPending(null)}
            >
                {pending ? t('确定要执行「{action}」吗？', { action: pending.label }) : ''}
            </Modal>
        </div>
    );
}
