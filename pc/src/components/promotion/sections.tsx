'use client';

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { fetchGrouponList, fetchSeckillList, type GrouponItem, type SeckillItem } from '@/lib/promotion';
import { GrouponCard, SeckillCard } from '@/components/promotion/cards';

/**
 * 首页/活动页的秒杀、拼团楼层。
 * Vue 基准：首页装修模块 `pc-home-seckill` / `pc-home-groupon`（PcHomeSeckill / PcHomeGroupon）。
 *
 * 装修数据里只有标题、倒计时文案等配置（module.title.titleText、countdownLabel），
 * **商品本身要另外取**：秒杀走 `home/home/getSeckill`，拼团走 `promotion/groupon/itemList`。
 * 楼层固定取前 limit 个，与 Vue 首页一致。
 */
interface SectionProps {
    title?: string;
    countdownLabel?: string;
    moreHref?: string;
    limit?: number;
}

function SectionShell({
    title,
    countdownLabel,
    moreHref,
    children,
}: SectionProps & { children: React.ReactNode }) {
    const { t } = useTranslation();
    return (
        <section className="pc-promo-section">
            <div className="pc-promo-section-head">
                <div className="pc-promo-section-title">
                    <h3>{title || t('限时秒杀')}</h3>
                    {countdownLabel ? <span className="pc-muted">{countdownLabel}</span> : null}
                </div>
                {moreHref ? (
                    <a className="pc-promo-more" href={moreHref}>
                        {t('更多')} →
                    </a>
                ) : null}
            </div>
            {children}
        </section>
    );
}

export function SeckillSection({ title, countdownLabel, moreHref = '/seckill/list', limit = 5 }: SectionProps) {
    const { t } = useTranslation();
    const [rows, setRows] = useState<SeckillItem[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchSeckillList(1, limit)
            .then((result) => setRows((result.records ?? []).slice(0, limit)))
            .catch(() => setRows([]))
            .finally(() => setLoading(false));
    }, [limit]);

    if (loading || rows.length === 0) return null;

    return (
        <SectionShell title={title} countdownLabel={countdownLabel} moreHref={moreHref}>
            <div className="pc-promo-grid">
                {rows.map((item) => (
                    <SeckillCard key={item.seckillId ?? item.productId} item={item} />
                ))}
            </div>
            {rows.length === 0 ? <div className="pc-muted">{t('暂无秒杀商品')}</div> : null}
        </SectionShell>
    );
}

export function GrouponSection({ title, countdownLabel, moreHref = '/joining/list', limit = 5 }: SectionProps) {
    const [rows, setRows] = useState<GrouponItem[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchGrouponList(1, limit)
            .then((result) => setRows((result.records ?? []).slice(0, limit)))
            .catch(() => setRows([]))
            .finally(() => setLoading(false));
    }, [limit]);

    if (loading || rows.length === 0) return null;

    return (
        <SectionShell title={title} countdownLabel={countdownLabel} moreHref={moreHref}>
            <div className="pc-promo-grid">
                {rows.map((item) => (
                    <GrouponCard key={item.grouponPromotionItemId ?? item.productId} item={item} />
                ))}
            </div>
        </SectionShell>
    );
}
