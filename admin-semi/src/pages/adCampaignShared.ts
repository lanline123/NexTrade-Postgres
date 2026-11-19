/**
 * 广告投流（ad_campaign）前端共享口径：商家「我的推广」、平台「投流管理」、
 * 平台「投流定价」三页共用。后端契约见 rust/crates/core/src/models/merchant/
 * ad_campaign.rs 与 services/src/system/ad_settings.rs：
 * - 投放单 VO 字段与 AdCampaign 实体同名（camelCase），金额 Decimal 以字符串下发；
 * - 时间字段（startTime/endTime/addTime…）为 Unix 秒；
 * - 生命周期不落库，由 pay_status / audit_status / status + 时间窗派生，
 *   lifecycleOf 与后端 AdCampaign::lifecycle 同口径，前端只做展示映射。
 */

/** 投放单（GET myList / auditList 的 records 项；POST create/audit 等的返回）。 */
export interface AdCampaign {
  id: number;
  /** 1 店铺 / 2 商品 */
  targetType: number;
  shopId: number;
  /** 商品投放时有值，店铺投放为 0 */
  productId: number;
  position: string;
  /** 1 = 平台置顶（免费运营位） */
  isPin: number;
  pinSort: number;
  startTime: number;
  endTime: number;
  days: number;
  /** 单价快照（元/天），Decimal 字符串 */
  unitPrice: string | number;
  /** 总价，Decimal 字符串 */
  amount: string | number;
  weight: number;
  /** 0 未付 / 1 已付 / 2 已退款 */
  payStatus: number;
  payChannel: string;
  payTime: number;
  accountLogId: number;
  /** 0 待审 / 1 通过 / 2 驳回 */
  auditStatus: number;
  auditRemark: string;
  auditAdminId: number;
  auditTime: number;
  /** 0 待生效 / 1 投放中 / 2 已暂停 / 3 已结束 / 4 已下架 */
  status: number;
  addTime: number;
  updateTime: number;
  /** 平台列表展示用（后端 VO 未回填时为空，前端回退显示 shopId） */
  shopTitle?: string;
  productName?: string;
}

/** 投流配置（config 表 adSettings 整块 JSON，GET/POST /setting/config/(save)AdSettings）。 */
export interface AdSettings {
  prices: Record<string, number>;
  minDays: number;
  pinLimit: number;
  promotedPerPage: number;
  recommendEnabled: boolean;
  needAudit: boolean;
}

/** 投放位清单（与后端 POSITIONS 同序同值；店铺投放只能 shop_street）。 */
export const AD_POSITIONS: Array<{ value: string; label: string }> = [
  { value: 'shop_street', label: '店铺街' },
  { value: 'product_search', label: '商品搜索结果' },
  { value: 'product_list', label: '商品列表' },
  { value: 'recommend_feed', label: '推荐流' },
];

export function positionLabel(position: string): string {
  return AD_POSITIONS.find((p) => p.value === position)?.label || position;
}

export function targetLabel(c: Pick<AdCampaign, 'targetType' | 'isPin'>): string {
  if (c.isPin === 1) return '店铺（置顶）';
  return c.targetType === 2 ? '商品' : '店铺';
}

/** 生命周期（与后端 Lifecycle 派生同口径，仅展示用）。 */
export type AdLifecycle =
  | 'pendingPay'
  | 'pendingAudit'
  | 'scheduled'
  | 'active'
  | 'paused'
  | 'ended'
  | 'rejected'
  | 'takenDown';

export function lifecycleOf(c: AdCampaign, nowSec = Math.floor(Date.now() / 1000)): AdLifecycle {
  if (c.status === 4) return 'takenDown';
  if (c.auditStatus === 2) return 'rejected';
  if (c.payStatus === 0) return 'pendingPay';
  if (c.payStatus === 2) return 'ended';
  if (c.auditStatus === 0) return 'pendingAudit';
  if (c.status === 2) return 'paused';
  if (c.status === 3 || nowSec >= c.endTime) return 'ended';
  if (nowSec < c.startTime) return 'scheduled';
  return 'active';
}

export const LIFECYCLE_META: Record<AdLifecycle, { label: string; color: string }> = {
  pendingPay: { label: '待付款', color: 'orange' },
  pendingAudit: { label: '待审核', color: 'amber' },
  scheduled: { label: '待生效', color: 'blue' },
  active: { label: '投放中', color: 'green' },
  paused: { label: '已暂停', color: 'grey' },
  ended: { label: '已结束', color: 'grey' },
  rejected: { label: '已驳回', color: 'red' },
  takenDown: { label: '已下架', color: 'red' },
};

export const AUDIT_META: Record<number, { label: string; color: string }> = {
  0: { label: '待审核', color: 'amber' },
  1: { label: '已通过', color: 'green' },
  2: { label: '已驳回', color: 'red' },
};

/** Unix 秒 → 本地时间文本（0/空显示 --）。 */
export function fmtTime(sec?: number): string {
  const n = Number(sec || 0);
  if (!n) return '--';
  const d = new Date(n * 1000);
  const pad = (v: number) => String(v).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Decimal 字符串/数字 → ¥金额文本。 */
export function fmtMoney(v?: string | number): string {
  const n = Number(v);
  return Number.isFinite(n) ? `¥${n.toFixed(2)}` : '--';
}

/** 投放时段文本：开始 ~ 结束。 */
export function fmtPeriod(c: Pick<AdCampaign, 'startTime' | 'endTime'>): string {
  return `${fmtTime(c.startTime)} ~ ${fmtTime(c.endTime)}`;
}
