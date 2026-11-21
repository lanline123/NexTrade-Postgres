import type { Metadata } from 'next';
import { cookies } from 'next/headers';
import Link from 'next/link';
import PcHeader from '@/components/PcHeaderWithData';
import PcFooter from '@/components/PcFooter';
import ShopInfoCard from '@/components/ShopInfoCard';
import ShopProductList from '@/components/ShopProductList';
import { getPcHeader, getShopList, type ShopListItem } from '@/lib/api';

import { getServerT, serverDecorateLocaleCode } from '@/i18n/server';
export const dynamic = 'force-dynamic';

/**
 * 店铺街 /shop（兼容路由 /shop/list）。
 * 源码对照：view/pc/pages/shop/List.vue
 * 样式：app/shop-page.scss → .pg-shop-list-page
 *
 * 旧版结构（List.vue <template>）：
 *   .shop-list-page
 *     > 页头（CommonPageHeader defaultSearchType="shop" + CommonHeader "店铺街"）
 *     > CommonPageContainer > .container
 *         > .filter-tabs > .tab-item(.active) 综合 / 评分
 *         > .shop-list-wrapper
 *             > .shop-list-container > .shop-row × N
 *                 > ShopInfo（.shop-info-card）+ ShopList（.shop-product-list）
 *             > .el-page > 分页
 *             > el-empty（暂无店铺数据）
 *     > 页脚
 *
 * 与旧版唯一保留的差异：旧版店铺搜索框由页头 defaultSearchType="shop" 承担，
 * 我们的 PcHeader 目前没有把该开关透传下去，所以页内保留一个纯表单搜索（不加自造类名）。
 *
 * 本平台新增：默认排序为智能推荐（sortField=recommend，置顶/推广/千人千面），
 * Tab 新增「推荐」并设为默认；「综合」(sort=comprehensive) 与「评分」
 * (sort=shopStar) 仍走旧排序。卡片角标（精选/推广）在 ShopInfoCard 内渲染。
 * recommend 时从 `accessToken` cookie 透传登录态（见下方 authToken），
 * 使 SSR 也能拿到 user_id 做千人千面；游客与其它排序不受影响。
 */
export async function generateMetadata(): Promise<Metadata> {
  const t = getServerT();
  return {
    title: t('店铺街 · 万物新生'),
    description: t('平台入驻店铺列表，查看店铺热销商品。'),
  };
}

interface PageProps {
  searchParams?: { [key: string]: string | string[] | undefined };
}

function readParam(value: string | string[] | undefined): string {
  return Array.isArray(value) ? value[0] || '' : value || '';
}

function shopHref(shop: ShopListItem): string {
  return `/shop/${shop.shopId}`;
}

export default async function ShopListPage({ searchParams }: PageProps) {
  const t = getServerT();
  const page = Math.max(1, Number(readParam(searchParams?.page)) || 1);
  const size = 10;
  const keyword = readParam(searchParams?.keyword);
  // 排序口径（URL 参数 sort 与旧版衔接）：
  // - 无 sort / sort=recommend ⇒ 默认智能推荐（后端 sortField=recommend：
  //   置顶 → 推广 → 千人千面 → 质量兜底）
  // - sort=comprehensive ⇒ 旧综合排序（不传 sortField，走后端原 SQL 分页）
  // - sort=shopStar ⇒ 旧评分排序
  const sortParam = readParam(searchParams?.sort);
  const sortMode = sortParam === 'shopStar' ? 'shopStar' : sortParam === 'comprehensive' ? 'comprehensive' : 'recommend';
  const sortField = sortMode === 'recommend' ? 'recommend' : sortMode === 'shopStar' ? 'shopStar' : undefined;
  // 登录态透传（照 middleware / MemberAuthGuard 的既有口径）：登录时 token 同时写入
  // localStorage 与 `accessToken` cookie（7 天），SSR 在服务端只能读 cookie。
  // 只在 recommend 排序时透传给店铺街接口供千人千面打分；无 cookie 即游客，
  // 综合/评分/keyword 行为完全不变。
  const authToken = sortField === 'recommend' ? cookies().get('accessToken')?.value || undefined : undefined;
  const [data, header] = await Promise.all([
    getShopList({ page, size, keyword, sortField, authToken }).catch(() => ({ records: [], total: 0 })),
    getPcHeader(undefined, serverDecorateLocaleCode()).catch(() => null),
  ]);
  const pages = Math.max(1, Math.ceil(data.total / size));
  const hrefFor = (nextPage: number, nextSortMode: string = sortMode) => {
    const query = new URLSearchParams();
    if (keyword) query.set('keyword', keyword);
    if (nextSortMode === 'shopStar') query.set('sort', 'shopStar');
    else if (nextSortMode === 'comprehensive') query.set('sort', 'comprehensive');
    if (nextPage > 1) query.set('page', String(nextPage));
    return query.toString() ? `/shop?${query.toString()}` : '/shop';
  };

  return (
    <div className="pg-shop-list-page">
      <div className="shop-list-page">
        <PcHeader config={header as any} />
        <main className="container">
          <div className="filter-tabs">
            <Link href={hrefFor(1, 'recommend')} className={`tab-item${sortMode === 'recommend' ? ' active' : ''}`}>
              {t('推荐')}
            </Link>
            <Link href={hrefFor(1, 'comprehensive')} className={`tab-item${sortMode === 'comprehensive' ? ' active' : ''}`}>
              {t('综合')}
            </Link>
            <Link href={hrefFor(1, 'shopStar')} className={`tab-item${sortMode === 'shopStar' ? ' active' : ''}`}>
              {t('评分')}
            </Link>
            <form action="/shop" method="get">
              <input name="keyword" defaultValue={keyword} placeholder={t('搜索店铺')} />
              {sortParam ? <input type="hidden" name="sort" value={sortParam} /> : null}
              <button type="submit">{t('搜索')}</button>
            </form>
          </div>

          <div className="shop-list-wrapper">
            {data.records.length ? (
              <>
                <div className="shop-list-container">
                  {data.records.map((shop) => (
                    <div className="shop-row" key={shop.shopId}>
                      <ShopInfoCard item={shop} />
                      <ShopProductList products={shop.listingProduct || []} />
                    </div>
                  ))}
                </div>

                {pages > 1 ? (
                  <div className="el-page">
                    <nav className="el-pagination">
                      {page > 1 ? (
                        <a href={hrefFor(page - 1)} className="btn-prev">
                          {'<'}
                        </a>
                      ) : (
                        <span className="btn-prev disabled">{'<'}</span>
                      )}
                      {Array.from({ length: Math.min(pages, 8) }).map((_, index) => {
                        const next = index + 1;
                        return next === page ? (
                          <span className="number active" key={next}>
                            {next}
                          </span>
                        ) : (
                          <a href={hrefFor(next)} className="number" key={next}>
                            {next}
                          </a>
                        );
                      })}
                      {pages > 8 ? <span className="more">…</span> : null}
                      {page < pages ? (
                        <a href={hrefFor(page + 1)} className="btn-next">
                          {'>'}
                        </a>
                      ) : (
                        <span className="btn-next disabled">{'>'}</span>
                      )}
                    </nav>
                  </div>
                ) : null}
              </>
            ) : (
              <div className="el-empty">{t('暂无店铺数据')}</div>
            )}
          </div>
        </main>
        <PcFooter />
      </div>
    </div>
  );
}
