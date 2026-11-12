'use client';

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { productImage, formatPrice } from '@/lib/format';
import {
    maxGroupNum,
    soldPercentOf,
    splitCountdown,
    toTimestamp,
    type Countdown,
    type GrouponItem,
    type SeckillItem,
} from '@/lib/promotion';

/** 每秒刷新的倒计时：目标时间戳 -> Countdown。 */
export function useCountdown(target?: number | string | null): Countdown {
    const targetMs = toTimestamp(target ?? 0);
    const [state, setState] = useState<Countdown>(() => splitCountdown(targetMs ? targetMs - Date.now() : 0));

    useEffect(() => {
        if (!targetMs) {
            setState(splitCountdown(0));
            return;
        }
        const tick = () => setState(splitCountdown(targetMs - Date.now()));
        tick();
        const timer = setInterval(tick, 1000);
        return () => clearInterval(timer);
    }, [targetMs]);

    return state;
}

/** 倒计时数字块（天/时/分/秒），到点显示「已结束」。 */
export function CountdownBoxes({ cd, endedText }: { cd: Countdown; endedText: string }) {
    if (cd.remain <= 0) return <span className="pc-cd-ended">{endedText}</span>;
    return (
        <span className="pc-cd">
            {cd.days > 0 ? (
                <>
                    <em>{cd.days}</em>
                    <i>天</i>
                </>
            ) : null}
            <em>{String(cd.hours).padStart(2, '0')}</em>
            <i>:</i>
            <em>{String(cd.minutes).padStart(2, '0')}</em>
            <i>:</i>
            <em>{String(cd.seconds).padStart(2, '0')}</em>
        </span>
    );
}

const STAMP =
    '?x-oss-process=image/resize,m_pad,h_300,h_300';

export function SeckillCard({ item }: { item: SeckillItem }) {
    const { t } = useTranslation();
    const cd = useCountdown(item.seckillEndTime ? Number(item.seckillEndTime) * 1000 : 0);
    const percent = soldPercentOf(item);
    const soldOut = Number(item.seckillStock ?? 0) <= 0;
    const notStarted = item.started === false;
    const href = `/product/${item.productSn ?? item.productId ?? ''}`;
    const thumb = item.picThumb || (item.picUrl ? `${item.picUrl}${STAMP}` : '');

    return (
        <a className={`pc-promo-card${soldOut ? ' is-soldout' : ''}`} href={href}>
            <div className="pc-promo-pic">
                <img src={thumb || productImage(item)} alt={item.productName ?? ''} loading="lazy" />
                {soldOut ? <span className="pc-promo-badge">{t('已抢完')}</span> : null}
                {!soldOut && notStarted ? <span className="pc-promo-badge">{t('即将开始')}</span> : null}
            </div>
            <div className="pc-promo-name">{item.productName}</div>
            <div className="pc-promo-prices">
                <span className="pc-promo-price">{formatPrice(item.seckillPrice)}</span>
                <span className="pc-promo-origin">{formatPrice(item.productPrice)}</span>
            </div>
            <div className="pc-promo-progress">
                <div className="pc-promo-progress-bar">
                    <i style={{ width: `${percent}%` }} />
                </div>
                <span>
                    {t('已售')}
                    {percent}%
                </span>
            </div>
            <div className="pc-promo-foot">
                {notStarted ? (
                    <span>
                        {t('距开始')} <CountdownBoxes cd={cd} endedText={t('已结束')} />
                    </span>
                ) : (
                    <span>
                        {t('距结束')} <CountdownBoxes cd={cd} endedText={t('已结束')} />
                    </span>
                )}
            </div>
        </a>
    );
}

export function GrouponCard({ item }: { item: GrouponItem }) {
    const { t } = useTranslation();
    const cd = useCountdown(item.endTime);
    const people = maxGroupNum(item.groupNum) || Number(item.minGroupNum ?? 0);
    const joined = Number(item.joinNumCount ?? 0);
    const href = `/product/${item.productSn ?? item.productId ?? ''}`;
    const thumb = item.picThumb || (item.picUrl ? `${item.picUrl}${STAMP}` : '');

    return (
        <a className="pc-promo-card" href={href}>
            <div className="pc-promo-pic">
                <img src={thumb || productImage(item)} alt={item.productName ?? ''} loading="lazy" />
                {people > 0 ? (
                    <span className="pc-promo-badge">
                        {people}
                        {t('人团')}
                    </span>
                ) : null}
            </div>
            <div className="pc-promo-name">{item.productName}</div>
            <div className="pc-promo-prices">
                <span className="pc-promo-price">{formatPrice(item.minGrouponPrice)}</span>
                <span className="pc-promo-origin">{formatPrice(item.productPrice)}</span>
            </div>
            <div className="pc-promo-progress">
                <div className="pc-promo-progress-bar">
                    <i style={{ width: `${people > 0 ? Math.min(100, (joined / people) * 100) : 0}%` }} />
                </div>
                <span>
                    {t('已拼')}
                    {joined}
                    {people > 0 ? `/${people}` : ''}
                </span>
            </div>
            <div className="pc-promo-foot">
                {item.canJoin === false ? (
                    <span className="pc-promo-restrict" title={item.restrictTip || t('仅限指定会员参与')}>
                        {t('仅限会员')}
                    </span>
                ) : (
                    <span>
                        {t('距结束')} <CountdownBoxes cd={cd} endedText={t('已结束')} />
                    </span>
                )}
            </div>
        </a>
    );
}
