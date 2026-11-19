import Link from 'next/link';
import { getAuctionItems, getAuctionSessions } from '@/lib/api';
import { getServerT } from '@/i18n/server';
import AuctionListClient from '@/components/AuctionListClient';

export const dynamic = 'force-dynamic';

const TABS = [
  { key: 'all', label: '全部' },
  { key: 'preview', label: '即将开始' },
  { key: 'running', label: '竞价中' },
  { key: 'finished', label: '已结束' },
];

const SORTS = [
  { key: 'ending', label: '即将结束' },
  { key: 'newest', label: '最新上架' },
  { key: 'priceAsc', label: '价格从低到高' },
  { key: 'priceDesc', label: '价格从高到低' },
];

const PAGE_SIZE = 20;

function buildHref(params: { tab: string; sessionId?: number; sort: string; page: number }): string {
  const q = new URLSearchParams();
  if (params.tab !== 'all') q.set('tab', params.tab);
  if (params.sessionId) q.set('sessionId', String(params.sessionId));
  if (params.sort !== 'ending') q.set('sort', params.sort);
  if (params.page > 1) q.set('page', String(params.page));
  const s = q.toString();
  return s ? `/auction?${s}` : '/auction';
}

export default async function AuctionChannelPage({
  searchParams,
}: {
  searchParams?: Promise<Record<string, string | string[] | undefined>>;
}) {
  const t = await getServerT();
  const sp = (await searchParams) || {};
  const tab = typeof sp.tab === 'string' && TABS.some((x) => x.key === sp.tab) ? sp.tab : 'all';
  const sort = typeof sp.sort === 'string' && SORTS.some((x) => x.key === sp.sort) ? sp.sort : 'ending';
  const sessionId = typeof sp.sessionId === 'string' ? Number(sp.sessionId) || undefined : undefined;
  const page = Math.max(1, Number(typeof sp.page === 'string' ? sp.page : '1') || 1);

  const [sessions, data] = await Promise.all([
    getAuctionSessions().catch(() => []),
    getAuctionItems({ tab, sessionId, sort, page, pageSize: PAGE_SIZE }).catch(() => ({ records: [], total: 0 })),
  ]);
  const totalPages = Math.max(1, Math.ceil(data.total / PAGE_SIZE));

  return (
    <main className="auction-page container">
      <div className="auction-page__head">
        <h1>{t('拍卖专区')}</h1>
        <p className="auction-page__sub">{t('冻结保证金后即可参与竞价，出价最高者中标；未中标保证金自动解冻。')}</p>
      </div>

      {sessions.length ? (
        <div className="auction-sessions">
          <Link
            href={buildHref({ tab, sort, page: 1 })}
            className={`auction-session-chip${!sessionId ? ' is-active' : ''}`}
          >
            {t('全部场次')}
          </Link>
          {sessions.map((s) => (
            <Link
              key={s.sessionId}
              href={buildHref({ tab, sessionId: s.sessionId, sort, page: 1 })}
              className={`auction-session-chip${sessionId === s.sessionId ? ' is-active' : ''}`}
            >
              <span className="auction-session-chip__name">{s.sessionName}</span>
              <span className="auction-session-chip__time">
                {s.startTimeText} ~ {s.endTimeText}
              </span>
            </Link>
          ))}
        </div>
      ) : null}

      <div className="auction-toolbar">
        <div className="auction-tabs">
          {TABS.map((x) => (
            <Link
              key={x.key}
              href={buildHref({ tab: x.key, sessionId, sort, page: 1 })}
              className={`auction-tab${tab === x.key ? ' is-active' : ''}`}
            >
              {t(x.label)}
            </Link>
          ))}
        </div>
        <div className="auction-sorts">
          {SORTS.map((x) => (
            <Link
              key={x.key}
              href={buildHref({ tab, sessionId, sort: x.key, page: 1 })}
              className={`auction-sort${sort === x.key ? ' is-active' : ''}`}
            >
              {t(x.label)}
            </Link>
          ))}
        </div>
      </div>

      <AuctionListClient records={data.records} />

      {totalPages > 1 ? (
        <div className="auction-pager">
          {page > 1 ? (
            <Link href={buildHref({ tab, sessionId, sort, page: page - 1 })}>{t('上一页')}</Link>
          ) : null}
          <span>
            {page} / {totalPages}
          </span>
          {page < totalPages ? (
            <Link href={buildHref({ tab, sessionId, sort, page: page + 1 })}>{t('下一页')}</Link>
          ) : null}
        </div>
      ) : null}
    </main>
  );
}
