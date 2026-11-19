'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';
import {
  fetchMyAuctionDeposits,
  fetchMyAuctionJoins,
  formatCountdown,
  moneyNum,
  type AuctionDeposit,
  type AuctionMyJoin,
} from '@/lib/auction';

type Tab = 'joins' | 'deposits' | 'penalty';

const DEPOSIT_STATUS_TEXT: Record<number, string> = {
  0: '未冻结',
  1: '冻结中',
  2: '已解冻',
  3: '已抵扣',
  4: '已扣罚',
  5: '部分扣罚后退回',
};

export default function MyAuctionClient() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>('joins');
  const [joins, setJoins] = useState<AuctionMyJoin[]>([]);
  const [deposits, setDeposits] = useState<AuctionDeposit[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    let hasToken = false;
    try {
      hasToken = Boolean(window.localStorage.getItem('accessToken'));
    } catch {
      hasToken = false;
    }
    if (!hasToken) {
      setError('unauthorized');
      setLoading(false);
      return;
    }
    (async () => {
      try {
        const [j, d] = await Promise.all([fetchMyAuctionJoins(), fetchMyAuctionDeposits(1, 50)]);
        setJoins(j);
        setDeposits(d.records);
      } catch (e) {
        setError(e instanceof Error ? e.message : t('加载失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  if (loading) return <div className="auction-empty">{t('加载中…')}</div>;
  if (error === 'unauthorized') {
    return (
      <div className="auction-empty">
        {t('请先登录')} ·{' '}
        <Link href={`/login?redirect=${encodeURIComponent('/member/auction')}`}>{t('去登录')}</Link>
      </div>
    );
  }
  if (error) return <div className="auction-empty">{error}</div>;

  const penaltyRecords = deposits.filter((d) => d.status === 4 || d.status === 5);

  return (
    <div className="my-auction">
      <div className="auction-tabs">
        <button type="button" className={`auction-tab${tab === 'joins' ? ' is-active' : ''}`} onClick={() => setTab('joins')}>
          {t('我的出价')}
        </button>
        <button
          type="button"
          className={`auction-tab${tab === 'deposits' ? ' is-active' : ''}`}
          onClick={() => setTab('deposits')}
        >
          {t('保证金记录')}
        </button>
        <button
          type="button"
          className={`auction-tab${tab === 'penalty' ? ' is-active' : ''}`}
          onClick={() => setTab('penalty')}
        >
          {t('违约记录')}
          {penaltyRecords.length ? <span className="my-auction__count">{penaltyRecords.length}</span> : null}
        </button>
      </div>

      {tab === 'joins' ? (
        joins.length ? (
          <div className="my-auction__list">
            {joins.map((j) => {
              const price = j.status === 3 && j.finalPrice ? j.finalPrice : j.currentPrice;
              const payRemain =
                j.myStatus === 'won' && j.payDeadline ? Math.max(0, j.payDeadline - Math.floor(Date.now() / 1000)) : null;
              return (
                <div key={j.itemId} className="my-auction__row">
                  <Link href={`/auction/${j.itemId}`} className="my-auction__thumb">
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img src={j.picThumb || j.picUrl} alt={j.title} />
                  </Link>
                  <div className="my-auction__body">
                    <Link href={`/auction/${j.itemId}`} className="my-auction__title">
                      {j.title}
                    </Link>
                    <div className="my-auction__meta">
                      <span>
                        {t('我的最高出价')} ¥{moneyNum(j.myHighestBid).toLocaleString()}
                      </span>
                      <span>
                        {j.status === 3 ? t('成交价') : t('当前价')} ¥{moneyNum(price).toLocaleString()}
                      </span>
                      <span>
                        {t('保证金')}：{t(DEPOSIT_STATUS_TEXT[j.myDepositStatus] || '-')}
                      </span>
                    </div>
                    {payRemain !== null ? (
                      <div className="my-auction__warn">
                        {t('请在')} {formatCountdown(payRemain)} {t('内完成支付，逾期将按规则扣罚保证金')}
                      </div>
                    ) : null}
                  </div>
                  <div className="my-auction__side">
                    <span className={`auction-tag is-${j.myStatus}`}>{j.myStatusName}</span>
                    {j.myStatus === 'won' && j.orderId ? (
                      <Link className="auction-btn is-primary" href={`/member/orders/${j.orderId}`}>
                        {t('去支付')}
                      </Link>
                    ) : j.status === 2 ? (
                      <Link className="auction-btn" href={`/auction/${j.itemId}`}>
                        {j.myStatus === 'outbid' ? t('去加价') : t('查看')}
                      </Link>
                    ) : (
                      <Link className="auction-btn" href={`/auction/${j.itemId}`}>
                        {t('查看')}
                      </Link>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="auction-empty">
            {t('暂无参拍记录')} · <Link href="/auction">{t('去拍卖专区看看')}</Link>
          </div>
        )
      ) : null}

      {tab === 'deposits' ? <DepositTable rows={deposits} t={t} emptyText={t('暂无保证金记录')} /> : null}
      {tab === 'penalty' ? (
        <>
          <DepositTable rows={penaltyRecords} t={t} emptyText={t('暂无违约扣罚记录')} />
          <p className="my-auction__note">{t('违约记录按保证金扣罚流水展示（已扣罚 / 部分扣罚后退回）。')}</p>
        </>
      ) : null}
    </div>
  );
}

function DepositTable({
  rows,
  t,
  emptyText,
}: {
  rows: AuctionDeposit[];
  t: (s: string) => string;
  emptyText: string;
}) {
  if (!rows.length) return <div className="auction-empty">{emptyText}</div>;
  return (
    <table className="my-auction__table">
      <thead>
        <tr>
          <th>{t('拍品')}</th>
          <th>{t('保证金金额')}</th>
          <th>{t('状态')}</th>
          <th>{t('扣罚金额')}</th>
          <th>{t('冻结时间')}</th>
          <th>{t('处理时间')}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((d) => (
          <tr key={d.depositId}>
            <td>
              <Link href={`/auction/${d.itemId}`}>{d.itemTitle}</Link>
            </td>
            <td>¥{moneyNum(d.amount).toLocaleString()}</td>
            <td>{d.statusName}</td>
            <td>{moneyNum(d.penaltyAmount) > 0 ? `¥${moneyNum(d.penaltyAmount).toLocaleString()}` : '-'}</td>
            <td>{d.addTimeText}</td>
            <td>{d.handleTimeText || '-'}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
