'use client';

import { useCallback, useEffect, useState } from 'react';
import { Button, Empty, Modal, Pagination, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiGet, type Paged } from '@/lib/client-api';
import { formatPrice, productImage } from '@/lib/format';

/**
 * 我的拼团。
 * Vue 基准：pages/member/joining/List.vue + components/member/joining/JoiningOrderList.vue（396 行）
 *
 * 真实接口：GET `promotion/groupon/record/list`
 *   参数 { page, size, sortField:'addTime', sortOrder:'desc', dateType:1, joining:1, isMine:true, joinType:1|2 }
 *   joinType 1 = 我发起的团，2 = 我参与的团
 *   记录 { joinStatus(1拼团中/2成功/3失败), orderId, orderSn, orderAddTime, quantity,
 *          grouponPrice, productName, productSn, picThumb, seoLink,
 *          grouponPromotionRecordId, minGroupNum, joinNumCount }
 *
 * 「邀请好友」在 Vue 里是弹二维码分享框；这里降级为复制邀请链接（内容一致：
 * `/joining/groupbyinvited?id=<recordId>`），不引入二维码生成依赖。
 */
interface GrouponRecord {
    joinStatus?: number;
    orderId?: number;
    orderSn?: string;
    orderAddTime?: string;
    quantity?: number;
    grouponPrice?: string | number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    picUrl?: string;
    grouponPromotionRecordId?: number;
    minGroupNum?: number;
    joinNumCount?: number;
}

const STATUS_TEXT: Record<number, string> = { 1: '拼团中', 2: '拼团成功', 3: '拼团失败' };
const STATUS_COLOR = { 1: 'blue', 2: 'green', 3: 'red' } as const;

export default function MyGrouponList() {
    const { t } = useTranslation();
    const [joinType, setJoinType] = useState(1);
    const [rows, setRows] = useState<GrouponRecord[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);
    const [inviting, setInviting] = useState<GrouponRecord | null>(null);

    const load = useCallback(async (nextPage: number, nextJoinType: number) => {
        setLoading(true);
        try {
            const result = await apiGet<Paged<GrouponRecord>>('promotion/groupon/record/list', {
                page: nextPage,
                size: 10,
                sortField: 'addTime',
                sortOrder: 'desc',
                dateType: 1,
                joining: 1,
                isMine: 'true',
                joinType: nextJoinType,
            });
            setRows(result.records ?? []);
            setTotal(Number(result.total ?? 0));
        } catch (e) {
            Toast.error((e as Error).message);
            setRows([]);
            setTotal(0);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        void load(page, joinType);
    }, [load, page, joinType]);

    const inviteLink = (row: GrouponRecord) =>
        typeof window === 'undefined'
            ? ''
            : `${window.location.origin}/joining/groupbyinvited?id=${row.grouponPromotionRecordId ?? ''}`;

    const copyInvite = async () => {
        if (!inviting) return;
        const link = inviteLink(inviting);
        try {
            await navigator.clipboard.writeText(link);
            Toast.success(t('邀请链接已复制'));
        } catch {
            Toast.info(link);
        }
        setInviting(null);
    };

    return (
        <div className="pc-member-panel">
            <div className="pc-panel-head">
                <Typography.Title heading={4}>{t('我的拼团')}</Typography.Title>
            </div>

            <div className="pc-tabs">
                {[
                    { type: 1, label: t('我发起的团') },
                    { type: 2, label: t('我参与的团') },
                ].map((item) => (
                    <button
                        key={item.type}
                        type="button"
                        className={joinType === item.type ? 'is-active' : ''}
                        onClick={() => {
                            setJoinType(item.type);
                            setPage(1);
                        }}
                    >
                        {item.label}
                    </button>
                ))}
            </div>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty
                    title={t('当前筛选下还没有任何拼团')}
                    description={<a href="/joining/list">{t('去参团')}</a>}
                />
            ) : (
                <div className="pc-card-grid">
                    {rows.map((row, index) => {
                        const need = Math.max(0, Number(row.minGroupNum ?? 0) - Number(row.joinNumCount ?? 0));
                        return (
                            <div className="pc-order-card" key={row.grouponPromotionRecordId ?? index}>
                                <div className="pc-row-head">
                                    <span className="pc-groupon-status">
                                        <Tag size="small" color={STATUS_COLOR[(row.joinStatus ?? 0) as keyof typeof STATUS_COLOR] ?? 'grey'}>
                                            {t(STATUS_TEXT[row.joinStatus ?? 0] ?? '未知')}
                                        </Tag>
                                        {row.joinStatus === 1 ? (
                                            <span className="pc-muted">
                                                （{t('还差')} {need} {t('人')}）
                                            </span>
                                        ) : null}
                                    </span>
                                    <a className="pc-link" href={`/member/order/info?id=${row.orderId ?? ''}`}>
                                        {t('订单详情')} →
                                    </a>
                                </div>

                                <div className="pc-muted" style={{ fontSize: 12 }}>
                                    {t('订单号')}：{row.orderSn ?? '-'} · {t('下单时间')}：{row.orderAddTime ?? '-'}
                                </div>

                                <div className="pc-order-item">
                                    <img src={productImage(row)} alt={row.productName ?? ''} />
                                    <div className="pc-order-item-main">
                                        <a className="pc-ellipsis" href={`/product/${row.productSn ?? ''}`}>
                                            {row.productName}
                                        </a>
                                        <div className="pc-muted">
                                            {t('拼团价')} {formatPrice(row.grouponPrice)}
                                        </div>
                                    </div>
                                    <div className="pc-muted">×{row.quantity ?? 1}</div>
                                </div>

                                <div className="pc-order-actions">
                                    {row.joinStatus === 1 ? (
                                        <Button size="small" theme="light" onClick={() => setInviting(row)}>
                                            {t('邀请好友')}
                                        </Button>
                                    ) : null}
                                    <a href={`/joining/groupbyinvited?id=${row.grouponPromotionRecordId ?? ''}`}>
                                        <Button size="small" theme="light">
                                            {t('拼团详情')}
                                        </Button>
                                    </a>
                                </div>
                            </div>
                        );
                    })}
                </div>
            )}

            {total > 10 ? (
                <div className="pc-pagination">
                    <Pagination currentPage={page} pageSize={10} total={total} onPageChange={setPage} />
                </div>
            ) : null}

            <Modal
                visible={!!inviting}
                title={t('邀请好友参团')}
                okText={t('复制链接')}
                onOk={() => void copyInvite()}
                onCancel={() => setInviting(null)}
            >
                <div className="pc-form">
                    <div className="pc-muted">{t('把链接发给好友，好友打开即可参团：')}</div>
                    <div className="pc-invite-link">{inviting ? inviteLink(inviting) : ''}</div>
                </div>
            </Modal>
        </div>
    );
}
