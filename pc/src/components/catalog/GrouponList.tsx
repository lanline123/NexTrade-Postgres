'use client';

import { useCallback, useEffect, useState } from 'react';
import { Empty, Pagination, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { fetchGrouponList, type GrouponItem } from '@/lib/promotion';
import { GrouponCard } from '@/components/promotion/cards';

/**
 * 拼团商品列表。
 * Vue 基准：pages/joining/List.vue + components/common/JoiningGoodsList.vue（416 行）
 *
 * 真实接口：`promotion/groupon/itemList?page=&size=`
 * （注意：拼团没有 `home/home/getGroupon`，那个 404；首页拼团模块也走 itemList）
 * 卡片口径：团价 / 原价划线 / 「N人团」（groupNum JSON 里最大档）/ 已拼 joinNumCount /
 * 剩余时间倒计时 / `canJoin === false` 显示「仅限会员」。
 */
const PAGE_SIZE = 20;

export default function GrouponList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<GrouponItem[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    const load = useCallback(async (nextPage: number) => {
        setLoading(true);
        try {
            const result = await fetchGrouponList(nextPage, PAGE_SIZE);
            setRows(result.records ?? []);
            setTotal(result.total ?? 0);
        } catch (e) {
            Toast.error((e as Error).message);
            setRows([]);
            setTotal(0);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        void load(page);
    }, [load, page]);

    return (
        <div className="pc-catalog pc-promo-page">
            <div className="pc-promo-head">
                <Typography.Title heading={3}>{t('拼团')}</Typography.Title>
                <span className="pc-muted">{t('凑够人数即可享受团购价')}</span>
            </div>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无拼团活动')} />
            ) : (
                <>
                    <div className="pc-promo-grid">
                        {rows.map((item) => (
                            <GrouponCard key={item.grouponPromotionItemId ?? item.productId} item={item} />
                        ))}
                    </div>
                    {total > PAGE_SIZE ? (
                        <div className="pc-pagination">
                            <Pagination
                                currentPage={page}
                                pageSize={PAGE_SIZE}
                                total={total}
                                onPageChange={setPage}
                            />
                        </div>
                    ) : null}
                </>
            )}
        </div>
    );
}
