/**
 * 装修页服务端首屏预取（SSR 注水）。
 *
 * 背景：pc-semi 的「商品 / 秒杀 / 拼团 / 优惠券 / 推荐 / 商品分组 / 通栏类目」等模块
 * 原本只在浏览器挂载后 fetch 首屏数据，服务端 HTML 只有模块壳与标题，卡片随后
 * 才出现并伴随高度塌陷。本文件在服务端渲染前，按与客户端 hook **完全相同的
 * 初始查询参数**（参数构建共用 ./prefetch-shared）把首屏数据取回，挂到
 * DecorateModule.__initialData 上随页面下发；客户端组件首屏直接用它渲染。
 *
 * 边界：
 * - 只取**匿名口径**数据：不转发 cookie / 登录态。含个人口径的模块在客户端
 *   挂载后仍会按原逻辑刷新（优惠券静默刷新领取状态；推荐的个性化流原地替换
 *   匿名首屏），视觉与数据口径与改造前一致，只是首屏不再空白。
 * - 任何一步失败 / 为空都返回 undefined（不挂 __initialData），客户端自动回落
 *   到原有取数路径，行为不比改造前差。
 * - 调用页面均为 force-dynamic，本文件不做跨请求缓存。
 */
import type { DecorateModule } from './types';
import { CONTRACT_ADAPTERS } from './components/contract';
import {
  apiQueryString,
  buildDecorateProductListQuery,
  contractRecommendMaxNum,
  contractRecommendPageParams,
  heroAllProductsQuery,
  heroGroupProductsQuery,
  productGroupColumns,
  productGroupTabQuery,
  pxNum,
  recommendPageParams,
} from './prefetch-shared';

const SERVER_API_BASE = process.env.PC_API_BASE || 'https://battery.recy.cc';
const FETCH_TIMEOUT_MS = 3000;

type AnyRecord = Record<string, any>;

/** 服务端取数：与客户端 apiGet 的 readData 同口径拆信封（裸数组接口原样返回）；异常返回 null */
async function fetchData(path: string): Promise<any> {
  try {
    const response = await fetch(`${SERVER_API_BASE}/api/${path}`, {
      cache: 'no-store',
      signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    });
    if (!response.ok) return null;
    const json: any = await response.json().catch(() => null);
    if (!json || typeof json !== 'object') return null;
    if ('code' in json && Number(json.code) !== 0) return null;
    if ('data' in json && json.data !== undefined) return json.data;
    return json;
  } catch {
    return null;
  }
}

/** 原始 fetch（不拆信封）：getCatFloor 的 icoDefinedCss 在信封顶层/ data 层都有可能出现 */
async function fetchRawJson(path: string): Promise<any> {
  try {
    const response = await fetch(`${SERVER_API_BASE}/api/${path}`, {
      cache: 'no-store',
      signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    });
    if (!response.ok) return null;
    return await response.json().catch(() => null);
  } catch {
    return null;
  }
}

/** product/list 首屏（与 useDecorateProductList 同一构建器）；null ⇔ 客户端本就不取数 */
async function prefetchProductList(
  config: AnyRecord,
  columns: number,
  shopId: unknown,
): Promise<ProductItemLike[] | undefined> {
  const query = buildDecorateProductListQuery(config, columns, shopId);
  if (query === null) return undefined;
  const data = await fetchData(`product/product/list?${query}`);
  if (data == null) return undefined;
  const records = (data as AnyRecord)?.records || data;
  return Array.isArray(records) ? records : undefined;
}

type ProductItemLike = AnyRecord;

/* ---------------- 各模块的预取口径（与 modules.tsx 客户端实现逐项对齐） ---------------- */

/** 商品（pc-product）/ 契约 Product / 分类商品简版·宽版：共享 hook 口径 */
async function prefetchViaProductHook(
  config: AnyRecord,
  columns: number,
  shopId: unknown,
): Promise<ProductItemLike[] | undefined> {
  return prefetchProductList(config || {}, columns, shopId);
}

/** 优惠券（pc-home-coupon）：home/home/getCoupon 无参，取 data（数组或 records），截前 4 */
async function prefetchCoupon(): Promise<any[] | undefined> {
  const data = await fetchData('home/home/getCoupon');
  if (data == null) return undefined;
  const list = Array.isArray(data) ? data : ((data as AnyRecord)?.records ?? []);
  return Array.isArray(list) ? list.slice(0, 4) : undefined;
}

/** 秒杀（pc-home-seckill）：getSeckill?page=1&size=10，严格 records 口径 */
async function prefetchSeckill(): Promise<any[] | undefined> {
  const data = await fetchData('home/home/getSeckill?page=1&size=10');
  if (data == null) return undefined;
  const records = (data as AnyRecord)?.records;
  return Array.isArray(records) ? records : undefined;
}

/** 拼团（pc-home-groupon）：无 promotionId 时客户端本就渲染为空；有则取 itemList 第 1 页 20 条 */
async function prefetchGroupon(module: AnyRecord): Promise<any[] | undefined> {
  const promotionId = module?.grouponActivityItem?.grouponPromotionId
    ? String(module.grouponActivityItem.grouponPromotionId)
    : '';
  if (!promotionId) return [];
  const data = await fetchData(
    `promotion/groupon/itemList?page=1&size=20&grouponPromotionId=${encodeURIComponent(promotionId)}`,
  );
  if (data == null) return undefined;
  const records = (data as AnyRecord)?.records;
  return Array.isArray(records) ? records : undefined;
}

/** 商品分组（pc-product-group）：只预取默认签（groups[0]），切签仍走客户端 */
async function prefetchProductGroup(module: AnyRecord): Promise<{ products: ProductItemLike[] } | undefined> {
  const groups: AnyRecord[] = Array.isArray(module.productGroups) ? module.productGroups : [];
  const group = groups[0];
  if (!group) return undefined;
  const columns = productGroupColumns(module);
  const query = productGroupTabQuery(group, columns);
  if (query === null) {
    // ids 分支且清单为空：客户端直接用清单兜底（空数组），无请求
    const fromList: AnyRecord[] = Array.isArray(group.productList) ? group.productList : [];
    return { products: fromList };
  }
  const data = await fetchData(`product/product/list?${query}`);
  if (data == null) {
    // ids 分支客户端在取数失败时回落清单商品；分组分支失败则保持空白（回落客户端重试）
    if (group.sourceType === 'productGroup' && group.productGroupId) return undefined;
    const fromList: AnyRecord[] = Array.isArray(group.productList) ? group.productList : [];
    return { products: fromList };
  }
  const records = (data as AnyRecord)?.records || data;
  return { products: Array.isArray(records) ? records : [] };
}

/**
 * 推荐（pc-recommend-product / 契约 RecommendProduct 同组件）：
 * - 非智能模式：与客户端 requestPage(1) 完全相同的参数取第 1 页；
 * - 智能模式：匿名复刻客户端 initAlgo 的首屏——无登录信号 ⇒ 无种子分类流，
 *   依次拉 cohort → hot → 普通兜底三条流（各第 1 页 24 条），按客户端同一
 *   去重 / 打分 / 排序规则入池，揭示首个 algoChunk 条作为匿名默认流。
 *   登录用户的个性化结果由客户端挂载后 initAlgo 原地替换。
 */
async function prefetchRecommend(module: AnyRecord): Promise<{ products: ProductItemLike[]; algo: boolean } | undefined> {
  const frame = module.frame || {};
  const config = module.products || {};
  const columns = Number(frame.columns) > 0 ? Number(frame.columns) : 5;
  const algo = (config.algorithm || {}) as AnyRecord;

  if (Number(algo.enabled) !== 1) {
    // 与客户端 requestPage(1) 同口径：第 1 页 size = min(20, maxNum)，maxNum<=0 时不取数
    const selectType = Number(config.productSelectType);
    const maxNum = selectType === 1
      ? (Array.isArray(config.productIds) ? config.productIds.length : 0)
      : Math.max(0, pxNum(config.productNumber, 0));
    if (maxNum <= 0) return { products: [], algo: false };
    const params = recommendPageParams(config, 1, Math.min(20, maxNum));
    const data = await fetchData(`product/product/list?${apiQueryString(params)}`);
    if (data == null) return undefined;
    const records = (data as AnyRecord)?.records;
    return { products: Array.isArray(records) ? records : [], algo: false };
  }

  const algoRows = Math.min(20, Math.max(1, Number(algo.defaultRows) || 2));
  const algoChunk = columns * algoRows;
  const seen = new Set<number>();
  const pool: ProductItemLike[] = [];
  const streams: Array<{ kind?: 'cohort' | 'hot' }> = [];
  if (Number(algo.useCohort ?? 1) === 1) streams.push({ kind: 'cohort' });
  if (Number(algo.useHot ?? 1) === 1) streams.push({ kind: 'hot' });
  streams.push({});

  for (const stream of streams) {
    let records: ProductItemLike[] = [];
    if (stream.kind === 'cohort') {
      const data = await fetchData('common/recommend/cohortBuys?page=1&size=24');
      records = Array.isArray(data) ? data : (data?.records || []);
    } else if (stream.kind === 'hot') {
      const data = await fetchData('common/recommend/guessLike?page=1&size=24&sortField=sales');
      records = Array.isArray(data) ? data : (data?.records || []);
    } else {
      const data = await fetchData('product/product/list?page=1&size=24');
      records = Array.isArray(data?.records) ? data.records : [];
    }
    const batch: Array<{ item: ProductItemLike; score: number }> = [];
    for (const item of Array.isArray(records) ? records : []) {
      const id = Number(item.productId);
      if (!id || seen.has(id)) continue;
      seen.add(id);
      const sales = Number(item.virtualSales ?? item.sales ?? 0) || 0;
      // 匿名无分类权重：cohort +60 / hot +30 / 普通 +0，再加销量分（与客户端公式一致）
      const streamBonus = stream.kind === 'cohort' ? 60 : stream.kind === 'hot' ? 30 : 0;
      batch.push({ item, score: streamBonus + Math.min(sales, 500) / 10 });
    }
    batch.sort((a, b) => b.score - a.score);
    for (const entry of batch) pool.push(entry.item);
    if (pool.length >= algoChunk) break;
  }
  return { products: pool.slice(0, algoChunk), algo: true };
}

/** 契约推荐（pcRecommendProduct → ContractRecommendProduct）：第 1 页（size 按 maxNum 推导） */
async function prefetchContractRecommend(module: AnyRecord): Promise<ProductItemLike[] | undefined> {
  const config = (module.products || {}) as AnyRecord;
  const maxNum = contractRecommendMaxNum(config);
  if (maxNum <= 0) return undefined;
  const params = contractRecommendPageParams(config, 1);
  if (params === null) return undefined;
  const data = await fetchData(`product/product/list?${apiQueryString(params)}`);
  if (data == null) return undefined;
  const records = (data as AnyRecord)?.records;
  return Array.isArray(records) ? records : undefined;
}

/**
 * 营销通栏（pc-home-hero）：中心卡商品在装修数据内（已 SSR），只补两处客户端取数：
 * 左侧类目楼层 getCatFloor（整包 JSON 下发，客户端用同一映射函数消费）与
 * productSource=productGroup 的卡位分组商品（按卡位下标对齐）；
 * productSource=all 的卡位（全部商品，我方新增）同位补拉无过滤前 2 个。
 */
async function prefetchHomeHero(
  module: AnyRecord,
): Promise<{ catRaw?: any; groupProducts?: Record<number, any[]> } | undefined> {
  const out: { catRaw?: any; groupProducts?: Record<number, any[]> } = {};
  const jobs: Array<Promise<void>> = [];

  jobs.push(
    (async () => {
      const raw = await fetchRawJson('common/pc/getCatFloor');
      if (raw && typeof raw === 'object') out.catRaw = raw;
    })(),
  );

  const centerCards: AnyRecord[] = Array.isArray(module.centerCards) ? module.centerCards : [];
  const leftCard: AnyRecord | undefined = module.leftCard;
  const allCards: AnyRecord[] = [...centerCards];
  if (leftCard && typeof leftCard === 'object' && Object.keys(leftCard).length > 0) allCards.push(leftCard);
  const groupProducts: Record<number, any[]> = {};
  allCards.forEach((card, idx) => {
    if (card?.productSource === 'productGroup' && card?.productGroupId) {
      jobs.push(
        (async () => {
          const query = heroGroupProductsQuery(card.productGroupId);
          const data = query ? await fetchData(`product/product/list?${query}`) : null;
          // 客户端口径：取数失败该卡位记空数组（渲染回落卡位自带商品）
          const records = (data as AnyRecord)?.records || data;
          groupProducts[idx] = Array.isArray(records) ? records : [];
        })(),
      );
    } else if (card?.productSource === 'all') {
      jobs.push(
        (async () => {
          const query = heroAllProductsQuery();
          const data = query ? await fetchData(`product/product/list?${query}`) : null;
          const records = (data as AnyRecord)?.records || data;
          groupProducts[idx] = Array.isArray(records) ? records : [];
        })(),
      );
    }
  });
  await Promise.all(jobs);
  if (Object.keys(groupProducts).length) out.groupProducts = groupProducts;
  return out.catRaw !== undefined || out.groupProducts ? out : undefined;
}

/* ---------------- 分发：与 DecorateRenderer 相同的两级查找 ---------------- */

async function prefetchForItem(item: DecorateModule): Promise<unknown> {
  const rawModule = (item.module || {}) as AnyRecord;

  // 1) 新版 kebab 注册表类型
  switch (item.type) {
    case 'pc-product': {
      const columns = Number(rawModule.style) === 2 ? 5 : 4;
      return prefetchViaProductHook(rawModule.products || {}, columns, rawModule.shopId);
    }
    case 'pc-home-coupon':
      return prefetchCoupon();
    case 'pc-home-seckill':
      return prefetchSeckill();
    case 'pc-home-groupon':
      return prefetchGroupon(rawModule);
    case 'pc-product-group':
      return prefetchProductGroup(rawModule);
    case 'pc-recommend-product':
      return prefetchRecommend(rawModule);
    case 'pc-home-hero':
      return prefetchHomeHero(rawModule);
    default:
      break;
  }

  // 2) 旧版 camelCase 契约类型：先经适配器得到渲染器键名 + 已转换 module，再按组件分发
  const adapter = CONTRACT_ADAPTERS[item.type];
  if (!adapter) return undefined;
  let plan;
  try {
    plan = adapter(item);
  } catch {
    return undefined;
  }
  const m = plan.module as AnyRecord;
  switch (plan.component) {
    case 'Product': {
      const columns = Number(m.style) === 2 ? 5 : 4;
      return prefetchViaProductHook(m.products || {}, columns, m.shopId);
    }
    case 'ContractCatProductSimple':
      return prefetchViaProductHook(m.product || {}, 4, m.shopId);
    case 'ContractCatProductWide':
      return prefetchViaProductHook(m.product || { productSelectType: 1, productIds: m.productIds }, 5, m.shopId);
    case 'Coupon':
      return prefetchCoupon();
    case 'Seckill':
      return prefetchSeckill();
    case 'Groupon':
      return prefetchGroupon(m);
    case 'RecommendProduct':
      return prefetchRecommend(m);
    case 'ContractRecommendProduct':
      return prefetchContractRecommend(m);
    default:
      return undefined;
  }
}

/**
 * 遍历页面模块并行预取首屏数据，成功者挂 __initialData 后返回新数组（不改原对象）。
 * 隐藏模块（isShow=false，渲染器本就过滤）跳过；单模块失败不影响其他模块。
 */
export async function prefetchDecorateModules(modules: DecorateModule[]): Promise<DecorateModule[]> {
  if (!Array.isArray(modules) || modules.length === 0) return modules;
  const results = await Promise.all(
    modules.map(async (item) => {
      if (!item || item.isShow === false) return undefined;
      try {
        return await prefetchForItem(item);
      } catch {
        return undefined;
      }
    }),
  );
  return modules.map((item, index) =>
    results[index] !== undefined ? { ...item, __initialData: results[index] } : item,
  );
}
