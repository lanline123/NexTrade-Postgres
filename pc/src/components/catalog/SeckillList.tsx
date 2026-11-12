'use client';

import { useCallback, useEffect, useState } from 'react';
import { Empty, Pagination, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { fetchSeckillList, type SeckillItem } from '@/lib/promotion';
import { SeckillCard } from '@/components/promotion/cards';

/**
 * 限时秒杀列表。
 * Vue 基准：pages/seckill/List.vue（370 行）
 *
 * 真实接口：`home/home/getSeckill?page=&size=`（不需要登录）。
 * 卡片口径照 Vue：秒杀价 / 原价划线 / 进度条（soldPercent）/ 剩余时间倒计时 /
 * `seckillStock<=0` 显示「已抢完」/ `started===false` 显示「即将开始」。
 * Vue 里那段被注释掉的时间轴 Tab 未迁（后端没给时段数据）。
 */
const PAGE_SIZE = 20;

export default function SeckillList() {
    const { t } = useTranslation();
    const [rows, setRows] = useState<SeckillItem[]>([]);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [loading, setLoading] = useState(true);

    const load = useCallback(async (nextPage: number) => {
        setLoading(true);
        try {
            const result = await fetchSeckillList(nextPage, PAGE_SIZE);
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
                <Typography.Title heading={3}>{t('限时秒杀')}</Typography.Title>
                <span className="pc-muted">{t('限时秒杀 抢先提醒')}</span>
            </div>

            {loading ? (
                <div className="pc-loading">{t('加载中')}</div>
            ) : rows.length === 0 ? (
                <Empty title={t('暂无秒杀商品')} />
            ) : (
                <>
                    <div className="pc-promo-grid">
                        {rows.map((item) => (
                            <SeckillCard key={item.seckillId ?? item.productId} item={item} />
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
