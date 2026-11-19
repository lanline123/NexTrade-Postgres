'use client';

import { useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import FormatPrice from '@/components/FormatPrice';
import { anchoredRemain, formatCountdown, moneyNum, type AuctionItem } from '@/lib/auction';
import { useTranslation } from 'react-i18next';

function statusBadge(item: AuctionItem): { text: string; cls: string } | null {
  switch (item.status) {
    case 1:
      return { text: '预告中', cls: 'is-preview' };
    case 2:
      return { text: '竞价中', cls: 'is-running' };
    case 3:
      return { text: '已成交', cls: 'is-sold' };
    case 4:
      return { text: '流拍', cls: 'is-unsold' };
    case 5:
      return { text: '已撤拍', cls: 'is-canceled' };
    case 6:
      return { text: '已结束', cls: 'is-canceled' };
    default:
      return null;
  }
}

export default function AuctionListClient({ records }: { records: AuctionItem[] }) {
  const { t } = useTranslation();
  const [now, setNow] = useState(() => Date.now());
  const anchorRef = useRef(Date.now());

  useEffect(() => {
    anchorRef.current = Date.now();
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [records]);

  if (!records.length) {
    return <div className="auction-empty">{t('暂无拍品')}</div>;
  }

  return (
    <div className="auction-grid">
      {records.map((item) => {
        const badge = statusBadge(item);
        const remain = anchoredRemain(item.remainSeconds, anchorRef.current, now);
        const isLive = item.status === 1 || item.status === 2;
        const urgent = item.status === 2 && remain <= 300;
        return (
          <Link key={item.itemId} href={`/auction/${item.itemId}`} className="auction-card">
            <div className="auction-card__img">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img src={item.picThumb || item.picUrl} alt={item.title} loading="lazy" />
              {badge ? <span className={`auction-badge ${badge.cls}`}>{t(badge.text)}</span> : null}
              {item.buyoutPrice ? <span className="auction-card__buyout">{t('一口价')}</span> : null}
            </div>
            <div className="auction-card__body">
              <div className="auction-card__title">{item.title}</div>
              <div className="auction-card__price-row">
                <span className="auction-card__price-label">
                  {item.status === 3 ? t('成交价') : item.hasBid ? t('当前价') : t('起拍价')}
                </span>
                <FormatPrice
                  value={moneyNum(item.status === 3 && item.finalPrice ? item.finalPrice : item.currentPrice)}
                  fontSize="20px"
                  fontWeight={700}
                />
              </div>
              <div className="auction-card__meta">
                <span>
                  {t('出价')} {item.bidCount} {t('次')}
                </span>
                {isLive ? (
                  <span className={`auction-card__count${urgent ? ' is-urgent' : ''}`}>
                    {item.status === 1 ? t('距开始') : t('距结束')} {formatCountdown(remain)}
                  </span>
                ) : item.status === 3 && item.winnerMaskedName ? (
                  <span>
                    {t('中标')} {item.winnerMaskedName}
                  </span>
                ) : null}
              </div>
            </div>
          </Link>
        );
      })}
    </div>
  );
}
