/**
 * 装修模块「首屏取数」的参数构建 —— 客户端组件与服务端预取共用同一份实现，
 * 保证两端发出的初始请求逐字一致（参数、顺序、空值处理都对齐原客户端代码）。
 *
 * 背景：pc-product / pc-home-seckill 等模块原本只在浏览器端 useEffect 里取数，
 * SSR 首屏只有标题和空壳，商品卡片等 fetch 回来才出现（高度塌陷 + 跳动）。
 * 服务端在渲染前用这里的构建器预取首屏数据并注水（见 server-prefetch.ts），
 * 客户端首屏直接用注水数据渲染，取数口径不变。
 */

/** 与 modules.tsx 的 px 同语义：Number 有限则取之，否则回退。 */
export function pxNum(value: unknown, fallback = 0): number {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

/**
 * useDecorateProductList（modules.tsx）的查询串。
 * 返回 null 表示客户端原本就不会发请求（size=0：未配置选品，列表恒为空）。
 */
export function buildDecorateProductListQuery(
  config: Record<string, any>,
  columns: number,
  shopId: unknown,
): string | null {
  const productSelectType = Number(config.productSelectType || 0);
  const params = new URLSearchParams({ page: '1' });
  if (productSelectType === 1) {
    const rawIds = config.productIds;
    const ids = Array.isArray(rawIds)
      ? rawIds
      : typeof rawIds === 'string' && rawIds
        ? rawIds.split(',').filter(Boolean)
        : [];
    if (ids.length) {
      params.set('ids', ids.join(','));
      params.set('size', String(ids.length));
    } else {
      params.set('size', '0');
    }
  } else if (productSelectType === 2) {
    params.set('categoryId', String(config.productCategoryId ?? ''));
    params.set('size', String(pxNum(config.productNumber, columns * 2)));
  } else if (productSelectType === 3) {
    params.set('introType', String(config.productTag ?? ''));
    params.set('size', String(pxNum(config.productNumber, columns * 2)));
  } else {
    params.set('size', '0');
  }
  if (Number(shopId) > 0) params.set('shopId', String(shopId));
  if (params.get('size') === '0') return null;
  return params.toString();
}

/** 客户端 apiGet（lib/client-api.ts）的查询串序列化：跳过 undefined / null / ''。 */
export function apiQueryString(params?: Record<string, unknown>): string {
  const query = new URLSearchParams();
  Object.entries(params || {}).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== '') query.set(key, String(value));
  });
  return query.toString();
}

/** RecommendProduct.buildParams：{page, size} + 按 selectType 附加 ids / categoryId / introType。 */
export function recommendPageParams(
  config: Record<string, any>,
  pageNo: number,
  size: number,
): Record<string, unknown> {
  const selectType = Number(config.productSelectType);
  const params: Record<string, unknown> = { page: pageNo, size };
  if (selectType === 1) {
    const ids = Array.isArray(config.productIds) ? config.productIds : [];
    params.ids = ids.join(',');
  } else if (selectType === 2) {
    params.categoryId = config.productCategoryId;
  } else if (selectType === 3) {
    params.introType = config.productTag;
  }
  return params;
}

/** ContractRecommendProduct 的 maxNum 口径：按 id 选时等于 id 个数，否则取 productNumber。 */
export function contractRecommendMaxNum(config: Record<string, any>): number {
  const selectType = Number(config.productSelectType) || 0;
  const ids: unknown[] = Array.isArray(config.productIds) ? config.productIds : [];
  return selectType === 1 ? ids.length : Math.max(0, pxNum(config.productNumber, 0));
}

/** ContractRecommendProduct.buildParams：size 按 sizeOf(page) 推导（每页 20，末页取余）。 */
export function contractRecommendPageParams(
  config: Record<string, any>,
  pageNo: number,
): Record<string, unknown> {
  const selectType = Number(config.productSelectType) || 0;
  const maxNum = contractRecommendMaxNum(config);
  const size = 20 * pageNo < maxNum ? 20 : Math.max(0, maxNum - 20 * (pageNo - 1));
  const params: Record<string, unknown> = { page: pageNo, size };
  if (selectType === 1) {
    const ids = Array.isArray(config.productIds) ? config.productIds : [];
    params.ids = ids.join(',');
  } else if (selectType === 2) {
    params.categoryId = config.productCategoryId;
  } else if (selectType === 3) {
    params.introType = config.productTag;
  }
  return params;
}

/** ProductGroup 取数列数（与组件内 listStyle → columns 口径一致）。 */
export function productGroupColumns(module: Record<string, any>): number {
  const listStyle = Number(module.listStyle ?? 4);
  return listStyle === 6 ? 5 : listStyle === 7 ? 6 : 4;
}

/**
 * ProductGroup 首个（或指定）分组的查询串，与组件 effect 内的构造逐字一致：
 * - sourceType=productGroup：page=1&size=columns*3&productGroupId=…
 * - 指定商品：ids=…&size=ids 个数（无 page 参数）
 * 返回 null 表示该分组不会发请求（无分组 / 指定商品为空）。
 */
export function productGroupTabQuery(
  group: Record<string, any> | undefined,
  columns: number,
): string | null {
  if (!group) return null;
  if (group.sourceType === 'productGroup' && group.productGroupId) {
    return new URLSearchParams({
      page: '1',
      size: String(columns * 3),
      productGroupId: String(group.productGroupId),
    }).toString();
  }
  const fromList = Array.isArray(group.productList) ? group.productList : [];
  const ids = fromList
    .map((item: any) => item.productId ?? item.id)
    .filter((id: any) => Boolean(id) && String(id) !== '0');
  if (!ids.length) return null;
  return new URLSearchParams({ ids: ids.join(','), size: String(ids.length) }).toString();
}

/** HomeHero 分组卡位取数查询串（page=1&size=2&productGroupId=…）。 */
export function heroGroupProductsQuery(productGroupId: unknown): string {
  return new URLSearchParams({
    page: '1',
    size: '2',
    productGroupId: String(productGroupId),
  }).toString();
}
