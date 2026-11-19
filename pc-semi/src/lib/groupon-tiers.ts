/**
 * 拼团阶梯档位解析（PC 与 H5 同口径；H5 对照 uniapp specification 的 laders 逻辑）。
 *
 * 数据来源：`promotion/groupon/getGrouponInfo` 返回的
 * - `groupNums`：[{ groupNumKey, groupNum }]（活动商品的档位表：key → 成团人数）
 * - `grouponPromotionDetails[].groupPrice`：JSON 串 [{ groupNumKey, price }]（该 SKU 各档团购价）
 *
 * 后端结算口径（check.rs `groupon_price_for` / `groupon_min_group_num`）：
 * 购物车 `grouponData.groupNumKey` 精确匹配档位价与成团人数；未传/未命中回落最低价档。
 * 默认选中最低价档、参团锁定该团成团人数对应档，均与 H5 一致。
 */
export interface GrouponTier {
  /** 档位 key（下单时随 grouponData.groupNumKey 下发） */
  key: number;
  /** 成团人数 */
  num: number;
  /** 该档团购价（当前 SKU） */
  price: number;
}

type AnyRecord = Record<string, any>;

/** 按当前 SKU 匹配拼团明细行（未命中回落首行，与详情页既有 buildGrouponData 口径一致）。 */
export function matchGrouponDetailRow(
  info: AnyRecord | null | undefined,
  skuId?: number,
): AnyRecord | undefined {
  const details = (info?.grouponPromotionDetails || []) as AnyRecord[];
  if (!details.length) return undefined;
  const matched = skuId ? details.find((row) => Number(row.skuId) === Number(skuId)) : undefined;
  return matched || details[0];
}

function parseGroupPriceEntries(raw: unknown): Array<{ key: number; price: number }> {
  let list: any = raw;
  if (typeof raw === 'string') {
    try {
      list = JSON.parse(raw);
    } catch {
      return [];
    }
  }
  if (!Array.isArray(list)) return [];
  const entries: Array<{ key: number; price: number }> = [];
  for (const item of list) {
    const key = Number(item?.groupNumKey);
    const price = Number(item?.price);
    if (Number.isFinite(key) && key > 0 && Number.isFinite(price)) entries.push({ key, price });
  }
  return entries;
}

/**
 * 解析某 SKU 明细行的阶梯档位（仅阶梯团 groupType=2 且档位完整时非空）。
 * 普通团返回空数组，调用方据此保持原有行为。
 */
export function resolveGrouponTiers(
  info: AnyRecord | null | undefined,
  detailRow: AnyRecord | undefined,
): GrouponTier[] {
  if (!info || Number(info.groupType) !== 2 || !detailRow) return [];
  const nums = (Array.isArray(info.groupNums) ? info.groupNums : []) as AnyRecord[];
  const numOf = (key: number) =>
    Number(nums.find((row) => Number(row?.groupNumKey) === key)?.groupNum || 0);
  const tiers = parseGroupPriceEntries(detailRow.groupPrice)
    .map((entry) => ({ key: entry.key, price: entry.price, num: numOf(entry.key) }))
    .filter((tier) => tier.num > 0);
  tiers.sort((a, b) => a.num - b.num || a.price - b.price);
  return tiers;
}

/** 默认档：最低价（同价取成团人数少者）——H5 默认选中最低价阶梯同口径。 */
export function defaultGrouponTier(tiers: GrouponTier[]): GrouponTier | undefined {
  if (!tiers.length) return undefined;
  return tiers.reduce((best, tier) =>
    tier.price < best.price || (tier.price === best.price && tier.num < best.num) ? tier : best,
  );
}

/** 按成团人数找档位（参团锁定：该团开团时选定的成团人数决定档位）。 */
export function grouponTierByNum(tiers: GrouponTier[], num: number): GrouponTier | undefined {
  if (!num) return undefined;
  return tiers.find((tier) => tier.num === num);
}
