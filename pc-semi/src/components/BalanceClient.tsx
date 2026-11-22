'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { Button, Card } from '@douyinfe/semi-ui';
import { apiGet } from '@/lib/client-api';

import { useTranslation } from 'react-i18next';
import { formatPrice } from '@/lib/currency';
interface AccountLog {
  logId?: number;
  /** 后端 UserAccountLogVO 的变动金额字段是 balance（旧字段名 amount 后端从未下发） */
  balance?: string | number;
  amount?: string;
  type?: string;
  postscript?: string;
  statusType?: string;
  /** 后端 UserAccountLogVO 的变动时间字段是 changeTime（旧字段名 addTime 后端从未下发） */
  changeTime?: string;
  addTime?: string;
  beforeBalance?: number;
  afterBalance?: number;
  changeTypeName?: string;
  changeDesc?: string;
}

const PAGE_SIZE = 30;

export default function BalanceClient() {
    const { t } = useTranslation();
  const [balance, setBalance] = useState(0);
  const [rows, setRows] = useState<AccountLog[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [needLogin, setNeedLogin] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    setLoading(true);
    setError('');
    Promise.all([
      apiGet<{ balance?: number }>('user/user/detail'),
      apiGet<{ records?: AccountLog[]; total?: number }>('user/account/list', { page, size: PAGE_SIZE }),
    ])
      .then(([profile, logs]) => {
        setBalance(Number(profile.balance || 0));
        setRows(logs.records || []);
        setTotal(Number(logs.total || 0));
      })
      .catch((error) => {
        const message = error instanceof Error ? error.message : '';
        if (message.includes('登录') || message.includes('401')) {
          setNeedLogin(true);
        } else {
          // 非登录类失败不能伪装成「暂无余额记录」，把错误显式展示
          setError(message || t('余额记录加载失败'));
        }
      })
      .finally(() => setLoading(false));
  }, [page]);

  if (needLogin) {
    return <Card className="cart-state"><h2>{t('登录后查看余额')}</h2><Link href="/login?redirect=/member/balance"><Button theme="solid" type="primary" size="large">{t('去登录')}</Button></Link></Card>;
  }

  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <div className="member-balance">
      <Card className="member-points__summary">
        <span>{t('账户余额')}</span>
        <b>{formatPrice(balance)}</b>
      </Card>
      <Card className="member-card" title={t('余额明细')}>
        {loading ? (
          <p>{t('余额记录加载中...')}</p>
        ) : error ? (
          <p>{error}</p>
        ) : rows.length ? (
          <div className="member-points__list">
            {rows.map((row, index) => {
              const amount = Number(row.balance ?? row.amount ?? 0);
              return (
                <div key={row.logId || index} className="member-points__row">
                  <div>
                    <b>{row.type || row.changeTypeName || row.changeDesc || t('余额变动')}</b>
                    <span>{row.changeTime || row.addTime} {row.postscript ? `· ${row.postscript}` : ''}</span>
                  </div>
                  <b className={amount >= 0 ? 'is-plus' : 'is-minus'}>
                    {amount >= 0 ? '+' : ''}{formatPrice(amount)}
                  </b>
                </div>
              );
            })}
          </div>
        ) : (
          <p>{t('暂无余额记录')}</p>
        )}
        {!loading && !error && total > PAGE_SIZE ? (
          <div className="el-page">
            <div className="pagination">
              <button type="button" disabled={page <= 1} aria-label={t('上一页')} onClick={() => setPage(page - 1)}>
                {'<'}
              </button>
              {Array.from({ length: pages }).map((_, index) => (
                <button key={index} type="button" className={index + 1 === page ? 'is-active' : ''} onClick={() => setPage(index + 1)}>
                  {index + 1}
                </button>
              ))}
              <button type="button" disabled={page >= pages} aria-label={t('下一页')} onClick={() => setPage(page + 1)}>
                {'>'}
              </button>
            </div>
          </div>
        ) : null}
      </Card>
    </div>
  );
}
