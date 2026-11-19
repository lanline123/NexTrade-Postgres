'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';
import FormatPrice from '@/components/FormatPrice';
import {
  anchoredRemain,
  fetchAuctionBids,
  fetchAuctionDetail,
  fetchAuctionPoll,
  formatCountdown,
  freezeAuctionDeposit,
  moneyNum,
  placeAuctionBid,
  quitAuction,
  remindAuction,
  type AuctionBid,
  type AuctionItemDetail,
} from '@/lib/auction';

function hasToken(): boolean {
  try {
    return Boolean(window.localStorage.getItem('accessToken'));
  } catch {
    return false;
  }
}

export default function AuctionDetailClient({ itemId, initial }: { itemId: number; initial: AuctionItemDetail }) {
  const { t } = useTranslation();
  const [detail, setDetail] = useState<AuctionItemDetail>(initial);
  const [loggedIn, setLoggedIn] = useState(false);
  const [bidInput, setBidInput] = useState('');
  const [busy, setBusy] = useState('');
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const anchorRef = useRef({ remain: initial.remainSeconds, at: Date.now() });
  const [extraBids, setExtraBids] = useState<AuctionBid[]>([]);
  const [bidPage, setBidPage] = useState(1);
  const [bidsDone, setBidsDone] = useState(false);

  const applyDetail = useCallback((d: AuctionItemDetail) => {
    setDetail(d);
    anchorRef.current = { remain: d.remainSeconds, at: Date.now() };
    setExtraBids([]);
    setBidPage(1);
    setBidsDone(false);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const d = await fetchAuctionDetail(itemId);
      applyDetail(d);
    } catch {
      /* 轮询失败静默，下轮重试 */
    }
  }, [itemId, applyDetail]);

  // 登录态下的完整详情（含我的保证金/出价状态）
  useEffect(() => {
    setLoggedIn(hasToken());
    void refresh();
  }, [refresh]);

  // 1 秒节拍驱动倒计时
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  // 竞价中/预告中：5 秒轮询
  useEffect(() => {
    if (detail.status !== 1 && detail.status !== 2) return;
    const timer = setInterval(async () => {
      try {
        const p = await fetchAuctionPoll(itemId);
        setDetail((prev) => ({
          ...prev,
          status: p.status,
          statusName: p.statusName,
          currentPrice: p.currentPrice,
          hasBid: p.hasBid,
          bidCount: p.bidCount,
          realEndTime: p.realEndTime,
          remainSeconds: p.remainSeconds,
          extendCount: p.extendCount,
          isLeader: p.isLeader,
          minNextBid: p.minNextBid,
          stepAmount: p.stepAmount,
          bids: p.latestBids.length ? p.latestBids : prev.bids,
        }));
        anchorRef.current = { remain: p.remainSeconds, at: Date.now() };
        if (p.status !== 1 && p.status !== 2) void refresh();
      } catch {
        /* 静默 */
      }
    }, 5000);
    return () => clearInterval(timer);
  }, [detail.status, itemId, refresh]);

  const remain = anchoredRemain(anchorRef.current.remain, anchorRef.current.at, now);
  const current = moneyNum(detail.hasBid || detail.status === 3 ? detail.currentPrice : detail.startPrice);
  const minNext = moneyNum(detail.minNextBid);
  const step = Math.max(1, moneyNum(detail.stepAmount));
  const deposit = moneyNum(detail.depositAmount);
  const frozen = detail.myDepositStatus === 1;
  const isRunning = detail.status === 2;
  const isPreview = detail.status === 1;
  const isFinished = detail.status >= 3;

  const bidValue = useMemo(() => {
    const n = Number(bidInput);
    return Number.isFinite(n) && n > 0 ? n : 0;
  }, [bidInput]);

  useEffect(() => {
    if (isRunning && !bidInput) setBidInput(String(minNext || ''));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isRunning, minNext]);

  const act = async (key: string, fn: () => Promise<void>) => {
    setBusy(key);
    setNotice(null);
    try {
      await fn();
    } catch (e) {
      setNotice({ ok: false, text: e instanceof Error ? e.message : t('操作失败，请重试') });
    } finally {
      setBusy('');
    }
  };

  const onFreeze = () =>
    act('freeze', async () => {
      await freezeAuctionDeposit(itemId);
      setNotice({ ok: true, text: t('保证金已冻结，可以出价了') });
      await refresh();
    });

  const submitBid = (price: number, isBuyout = false) =>
    act(isBuyout ? 'buyout' : 'bid', async () => {
      const market = moneyNum(detail.productPrice);
      const needConfirm = !isBuyout && market > 0 && price > market;
      if (needConfirm && !window.confirm(t('您的出价高于该商品市场价，确认继续出价吗？'))) return;
      const r = await placeAuctionBid(itemId, price, { isBuyout, confirmHigh: needConfirm });
      setNotice({
        ok: true,
        text: isBuyout || r.isBuyout ? t('一口价成交成功，请前往订单完成支付') : t('出价成功，您是当前最高出价者'),
      });
      await refresh();
    });

  const onQuit = () =>
    act('quit', async () => {
      if (!window.confirm(t('退出参拍后保证金将解冻退回，确认退出？'))) return;
      await quitAuction(itemId);
      setNotice({ ok: true, text: t('已退出参拍，保证金已解冻') });
      await refresh();
    });

  const onRemind = () =>
    act('remind', async () => {
      await remindAuction(itemId, !detail.reminded);
      setDetail((p) => ({ ...p, reminded: !p.reminded }));
    });

  const loadMoreBids = async () => {
    const next = bidPage + 1;
    const res = await fetchAuctionBids(itemId, next, 20).catch(() => null);
    if (!res || !res.records.length) {
      setBidsDone(true);
      return;
    }
    setExtraBids((p) => [...p, ...res.records]);
    setBidPage(next);
    if (res.records.length < 20) setBidsDone(true);
  };

  const bids = [...detail.bids, ...extraBids];

  return (
    <div className="auction-detail">
      <div className="auction-detail__main">
        <div className="auction-detail__gallery">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={detail.picUrl || detail.picThumb} alt={detail.title} />
          <span className={`auction-badge auction-detail__badge ${isRunning ? 'is-running' : isPreview ? 'is-preview' : 'is-sold'}`}>
            {detail.statusName}
          </span>
        </div>

        <div className="auction-detail__info">
          <h1>{detail.title}</h1>
          {detail.productBrief ? <p className="auction-detail__brief">{detail.productBrief}</p> : null}

          <div className="auction-detail__pricebox">
            <div className="auction-detail__price-main">
              <span>{isFinished && detail.status === 3 ? t('成交价') : detail.hasBid ? t('当前价') : t('起拍价')}</span>
              <FormatPrice value={current} fontSize="32px" fontWeight={700} />
              {detail.isLeader && isRunning ? <em className="auction-tag is-leader">{t('您是最高出价者')}</em> : null}
              {detail.isWinner ? <em className="auction-tag is-leader">{t('您已中标')}</em> : null}
            </div>
            <div className="auction-detail__price-sub">
              <span>
                {t('起拍价')} ¥{moneyNum(detail.startPrice).toLocaleString()}
              </span>
              <span>
                {t('市场价')} ¥{moneyNum(detail.productPrice).toLocaleString()}
              </span>
              <span>
                {t('出价')} {detail.bidCount} {t('次')}
              </span>
            </div>
            {isRunning || isPreview ? (
              <div className={`auction-detail__count${isRunning && remain <= 300 ? ' is-urgent' : ''}`}>
                {isPreview ? t('距开拍') : t('距结束')} <strong>{formatCountdown(remain)}</strong>
                {detail.extendCount > 0 ? (
                  <span className="auction-detail__extend">
                    {t('已延时')} {detail.extendCount} {t('次')}
                  </span>
                ) : null}
              </div>
            ) : (
              <div className="auction-detail__count">
                {detail.status === 3
                  ? `${t('已成交')}${detail.winnerMaskedName ? ` · ${t('中标')} ${detail.winnerMaskedName}` : ''}`
                  : detail.statusName}
              </div>
            )}
          </div>

          <dl className="auction-detail__facts">
            <div>
              <dt>{t('参拍保证金')}</dt>
              <dd>¥{deposit.toLocaleString()}</dd>
            </div>
            <div>
              <dt>{t('加价幅度')}</dt>
              <dd>¥{step.toLocaleString()}</dd>
            </div>
            <div>
              <dt>{t('开拍时间')}</dt>
              <dd>{detail.startTimeText}</dd>
            </div>
            <div>
              <dt>{t('结束时间')}</dt>
              <dd>{detail.endTimeText}</dd>
            </div>
            {detail.buyoutPrice ? (
              <div>
                <dt>{t('一口价')}</dt>
                <dd>¥{moneyNum(detail.buyoutPrice).toLocaleString()}</dd>
              </div>
            ) : null}
          </dl>

          {notice ? <div className={`auction-notice${notice.ok ? ' is-ok' : ' is-err'}`}>{notice.text}</div> : null}

          {/* ——— 操作区（状态机） ——— */}
          <div className="auction-actions">
            {!loggedIn && !isFinished ? (
              <div className="auction-guide">
                <p>{t('登录后冻结保证金即可参与竞价')}</p>
                <Link className="auction-btn is-primary" href={`/login?redirect=${encodeURIComponent(`/auction/${itemId}`)}`}>
                  {t('去登录')}
                </Link>
              </div>
            ) : detail.banReason ? (
              <div className="auction-guide is-blocked">
                <p>{detail.banReason}</p>
              </div>
            ) : isPreview ? (
              <div className="auction-guide">
                <p>
                  {t('将于')} {detail.startTimeText} {t('开拍')}
                  {loggedIn && !frozen ? `，${t('可提前冻结保证金')}` : ''}
                </p>
                {loggedIn && !frozen ? (
                  detail.requireCertification && !detail.myCertified ? (
                    <Link className="auction-btn is-primary" href="/member/certification">
                      {t('先完成企业认证')}
                    </Link>
                  ) : (
                    <button type="button" className="auction-btn is-primary" disabled={busy === 'freeze'} onClick={onFreeze}>
                      {t('冻结保证金')} ¥{deposit.toLocaleString()}
                    </button>
                  )
                ) : null}
                {loggedIn ? (
                  <button type="button" className="auction-btn" disabled={busy === 'remind'} onClick={onRemind}>
                    {detail.reminded ? t('取消开拍提醒') : t('开拍提醒')}
                  </button>
                ) : null}
                {frozen ? <span className="auction-tag">{t('保证金已冻结')}</span> : null}
              </div>
            ) : isRunning ? (
              detail.requireCertification && !detail.myCertified && !frozen ? (
                <div className="auction-guide is-blocked">
                  <p>{t('本场拍卖需完成企业认证后方可参拍')}</p>
                  <Link className="auction-btn is-primary" href="/member/certification">
                    {t('去认证')}
                  </Link>
                </div>
              ) : !frozen ? (
                <div className="auction-guide">
                  <p>
                    {t('参拍需先从余额冻结保证金')} ¥{deposit.toLocaleString()}，{t('未中标自动解冻')}
                  </p>
                  <button type="button" className="auction-btn is-primary" disabled={busy === 'freeze'} onClick={onFreeze}>
                    {t('冻结保证金并参拍')}
                  </button>
                </div>
              ) : detail.isLeader ? (
                <div className="auction-guide">
                  <p>
                    {t('您已出价')} ¥{moneyNum(detail.myHighestBid).toLocaleString()}，{t('当前领先')}
                  </p>
                  <button type="button" className="auction-btn" disabled={busy === 'quit'} onClick={onQuit}>
                    {t('退出参拍')}
                  </button>
                </div>
              ) : (
                <div className="auction-bidbox">
                  <div className="auction-bidbox__row">
                    <button
                      type="button"
                      className="auction-step-btn"
                      onClick={() => setBidInput(String(Math.max(minNext, bidValue - step)))}
                    >
                      −
                    </button>
                    <input
                      className="auction-bidbox__input"
                      inputMode="numeric"
                      value={bidInput}
                      onChange={(e) => setBidInput(e.target.value.replace(/[^0-9.]/g, ''))}
                      aria-label={t('出价金额')}
                    />
                    <button type="button" className="auction-step-btn" onClick={() => setBidInput(String(bidValue + step))}>
                      +
                    </button>
                    <button
                      type="button"
                      className="auction-btn is-primary"
                      disabled={busy === 'bid' || bidValue < minNext}
                      onClick={() => submitBid(bidValue)}
                    >
                      {t('出价')}
                    </button>
                  </div>
                  <div className="auction-bidbox__chips">
                    <button type="button" onClick={() => setBidInput(String(minNext))}>
                      {t('最低')} ¥{minNext.toLocaleString()}
                    </button>
                    <button type="button" onClick={() => setBidInput(String(minNext + step * 4))}>
                      +5{t('档')}
                    </button>
                    <button type="button" onClick={() => setBidInput(String(minNext + step * 9))}>
                      +10{t('档')}
                    </button>
                    {bidValue > 0 && bidValue < minNext ? (
                      <span className="auction-bidbox__warn">
                        {t('出价不能低于')} ¥{minNext.toLocaleString()}
                      </span>
                    ) : null}
                  </div>
                  <div className="auction-bidbox__row is-sub">
                    {detail.buyoutPrice ? (
                      <button
                        type="button"
                        className="auction-btn is-buyout"
                        disabled={busy === 'buyout'}
                        onClick={() => {
                          if (window.confirm(`${t('确认以一口价')} ¥${moneyNum(detail.buyoutPrice).toLocaleString()} ${t('直接成交？')}`)) {
                            void submitBid(moneyNum(detail.buyoutPrice), true);
                          }
                        }}
                      >
                        {t('一口价')} ¥{moneyNum(detail.buyoutPrice).toLocaleString()} {t('成交')}
                      </button>
                    ) : null}
                    <button type="button" className="auction-btn is-ghost" disabled={busy === 'quit'} onClick={onQuit}>
                      {t('退出参拍')}
                    </button>
                  </div>
                </div>
              )
            ) : (
              <div className="auction-guide">
                <p>
                  {detail.status === 3
                    ? detail.isWinner
                      ? `${t('恭喜中标！请在规定时间内完成订单支付')}。`
                      : t('本场拍卖已成交')
                    : t('本场拍卖已结束')}
                </p>
                {detail.orderId ? (
                  <Link className="auction-btn is-primary" href={`/member/orders/${detail.orderId}`}>
                    {t('查看订单')}
                  </Link>
                ) : null}
                <Link className="auction-btn" href="/member/auction">
                  {t('我的拍卖')}
                </Link>
              </div>
            )}
          </div>

          <div className="auction-rules">
            <h3>{t('竞拍规则')}</h3>
            <ul>
              <li>{t('参拍前需从账户余额冻结保证金，未中标在拍卖结束后自动解冻。')}</li>
              <li>{t('每次出价至少加一档加价幅度；结束前 5 分钟内有新出价，结束时间自动顺延 5 分钟。')}</li>
              <li>{t('中标后保证金抵扣货款，需在 48 小时内完成支付；逾期未付将按规则扣罚保证金。')}</li>
              <li>{t('出价后不可撤销；被更高出价超过时可继续加价。')}</li>
            </ul>
          </div>
        </div>
      </div>

      <div className="auction-detail__lower">
        <section className="auction-bidwall">
          <h2>
            {t('出价记录')}（{detail.bidCount}）
          </h2>
          {bids.length ? (
            <ul>
              {bids.map((b) => (
                <li key={b.bidId} className={b.isLeader ? 'is-leader' : ''}>
                  <span className="auction-bidwall__name">
                    {b.maskedName}
                    {b.isMine ? <em>（{t('我')}）</em> : null}
                    {b.isBuyout ? <em className="auction-tag is-buyout">{t('一口价')}</em> : null}
                  </span>
                  <span className="auction-bidwall__price">¥{moneyNum(b.bidPrice).toLocaleString()}</span>
                  <span className="auction-bidwall__time">{b.addTimeText}</span>
                  {b.isLeader ? <span className="auction-tag is-leader">{t('领先')}</span> : null}
                </li>
              ))}
            </ul>
          ) : (
            <p className="auction-empty">{t('暂无出价')}</p>
          )}
          {bids.length >= 20 && !bidsDone ? (
            <button type="button" className="auction-btn is-ghost" onClick={() => void loadMoreBids()}>
              {t('加载更多')}
            </button>
          ) : null}
        </section>

        {detail.productDesc ? (
          <section className="auction-desc">
            <h2>{t('商品详情')}</h2>
            <div dangerouslySetInnerHTML={{ __html: detail.productDesc }} />
          </section>
        ) : null}
      </div>
    </div>
  );
}
