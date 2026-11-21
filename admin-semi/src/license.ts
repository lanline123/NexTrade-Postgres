/**
 * 授权系统客户端（前端侧，我方新增）。
 *
 * 数据来自后端 `GET /adminapi/license/status`（本实例缓存的授权判定，
 * 判定源头是授权中心 auth.recy.cc 对 checkin 上报的验签响应；前端不直连
 * 授权中心）。本模块负责三件事：
 * - 判定归一化 + localStorage 缓存（首屏先用缓存渲染，避免菜单闪烁）；
 * - 授权徽标文案口径（组件 `LicensedBadge` 消费）；
 * - **菜单版本控制**：集中映射表 `MENU_MIN_TIER`（菜单 key → 最低档位），
 *   `filterNavGroupsByLicense` 在 AdminLayout 的菜单过滤链里消费。
 *
 * fail-soft 铁律：状态拉取失败 / 响应异常一律回落缓存或 unknown；
 * unknown 与未授权只影响「平台级菜单」的显隐与徽标提示，绝不拦截路由、
 * 绝不影响其余功能（与 federation 门控同为渲染层过滤）。
 */

import { apiGet } from './api';
import type { NavGroup, NavItem } from './config/adminMenu';

export type LicenseTier = 'commercial' | 'enterprise';
export type LicenseStatusKind = 'authorized' | 'unauthorized' | 'unknown';

export interface LicenseStatus {
  status: LicenseStatusKind;
  tier: LicenseTier | null;
  expiresAt: string | null;
  expired: boolean;
  subdomainLimit: number | null;
  subdomainsUsed: number | null;
  checkedAt: string | null;
  instanceKey: string | null;
}

export const LICENSE_CACHE_KEY = 'admin_license';

export const DEFAULT_LICENSE_STATUS: LicenseStatus = {
  status: 'unknown',
  tier: null,
  expiresAt: null,
  expired: false,
  subdomainLimit: null,
  subdomainsUsed: null,
  checkedAt: null,
  instanceKey: null,
};

function asRecord(value: unknown): Record<string, any> {
  return value && typeof value === 'object' ? (value as Record<string, any>) : {};
}

function asStr(value: unknown): string | null {
  return typeof value === 'string' && value.trim() !== '' ? value : null;
}

function asNum(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

/** 归一化后端状态口响应：字段缺失/非法一律回落 unknown 口径。 */
export function normalizeLicenseStatus(raw: unknown): LicenseStatus {
  const data = asRecord(raw);
  const status: LicenseStatusKind =
    data.status === 'authorized' || data.status === 'unauthorized' ? data.status : 'unknown';
  const tier: LicenseTier | null =
    data.tier === 'commercial' || data.tier === 'enterprise' ? data.tier : null;
  return {
    status,
    // 无档位的 authorized 不成立（授权中心契约：authorized 必带 tier），降级 unknown 防误开菜单
    tier,
    expiresAt: asStr(data.expiresAt),
    expired: data.expired === true,
    subdomainLimit: asNum(data.subdomainLimit),
    subdomainsUsed: asNum(data.subdomainsUsed),
    checkedAt: asStr(data.checkedAt),
    instanceKey: asStr(data.instanceKey),
  };
}

export function getCachedLicenseStatus(): LicenseStatus {
  try {
    const raw = localStorage.getItem(LICENSE_CACHE_KEY);
    return raw ? normalizeLicenseStatus(JSON.parse(raw)) : { ...DEFAULT_LICENSE_STATUS };
  } catch {
    return { ...DEFAULT_LICENSE_STATUS };
  }
}

export function cacheLicenseStatus(status: LicenseStatus) {
  try {
    localStorage.setItem(LICENSE_CACHE_KEY, JSON.stringify(normalizeLicenseStatus(status)));
  } catch {
    // localStorage 不可用时忽略，门控退回 unknown
  }
}

/** 拉取授权状态并缓存；失败时返回缓存值（默认 unknown）。 */
export async function fetchLicenseStatus(): Promise<LicenseStatus> {
  try {
    const data = await apiGet<Record<string, any>>('/license/status');
    const status = normalizeLicenseStatus(data);
    cacheLicenseStatus(status);
    return status;
  } catch {
    return getCachedLicenseStatus();
  }
}

/**
 * 当前档位等级（菜单门控用）：
 * - 0 = 未授权 / 未知 / 已过期（authorized 但 expired=true 视为失效）；
 * - 1 = 商业版；2 = 企业版。
 */
export function licenseRank(status: LicenseStatus): number {
  if (status.status !== 'authorized' || status.expired) return 0;
  if (status.tier === 'enterprise') return 2;
  if (status.tier === 'commercial') return 1;
  return 0;
}

/**
 * 菜单-最低档位映射表（**版本控制的唯一定义处**，调整版本划分只改这里）。
 *
 * key 以 `config/adminMenu.tsx` 的实名为准（组 key / 栏 key / 页面 key 三级
 * 都可登记；过滤时逐级匹配）。未登记的菜单 = 无最低档位，各档位（含未授权）
 * 都显示——基础版只圈平台级能力，其余经营功能不做版本划分。
 *
 * | key | 菜单位置 | 最低档位 | 说明 |
 * |-----|----------|----------|------|
 * | organize6 | 组织 → 万象互联（整栏） | enterprise | 联邦面整栏，商业版不含 |
 * | orgOverview / hubLinks / hubPairing / myLinks / hubCredentials / hubAggregation / hubMembers / hubOrders | 万象互联栏内各页 | enterprise | 与整栏同档，逐页登记防栏结构调整后漏网 |
 * | organize7 | 组织 → 商品流通（整栏） | enterprise | 与万象互联同属联邦面（federation 门控亦将两栏并列处理） |
 * | feedKeyManage | 商品流通 → 对接 API 管理 | enterprise | 同上 |
 * | federationSettings | 设置 → 商城设置 → 万象互联 | enterprise | 联邦设置入口，与栏同档 |
 * | siteClone | 设置 → 商城设置 → 站点搬家 | commercial | 平台级工具：未授权/未知隐藏，商业版起可用 |
 * | systemUpdate | 设置 → 系统设置 → 系统更新 | commercial | 平台级工具：未授权/未知隐藏，商业版起可用 |
 */
export const MENU_MIN_TIER: Record<string, LicenseTier> = {
  organize6: 'enterprise',
  orgOverview: 'enterprise',
  hubLinks: 'enterprise',
  hubPairing: 'enterprise',
  myLinks: 'enterprise',
  hubCredentials: 'enterprise',
  hubAggregation: 'enterprise',
  hubMembers: 'enterprise',
  hubOrders: 'enterprise',
  organize7: 'enterprise',
  feedKeyManage: 'enterprise',
  federationSettings: 'enterprise',
  siteClone: 'commercial',
  systemUpdate: 'commercial',
};

const TIER_RANK: Record<LicenseTier, number> = { commercial: 1, enterprise: 2 };

function requiredRank(key: string): number {
  const tier = MENU_MIN_TIER[key];
  return tier ? TIER_RANK[tier] : 0;
}

function filterItems(items: NavItem[], rank: number): NavItem[] {
  return items
    .filter((item) => rank >= requiredRank(item.key))
    .map((item) =>
      item.children
        ? { ...item, children: item.children.filter((child) => rank >= requiredRank(child.key)) }
        : item,
    )
    .filter((item) => !item.children || item.children.length > 0);
}

/**
 * 按授权档位过滤菜单树（仅渲染层，不拦路由直达，与 federation / systemMode
 * 门控同定位）：档位不足的菜单摘除，栏内子页摘空则整栏摘除，一级组摘空
 * 则整组摘除。企业版（rank 2）直接原样返回。
 */
export function filterNavGroupsByLicense(groups: NavGroup[], status: LicenseStatus): NavGroup[] {
  const rank = licenseRank(status);
  if (rank >= 2) return groups;
  return groups
    .map((group) => ({ ...group, items: filterItems(group.items, rank) }))
    .filter((group) => group.items.length > 0);
}
