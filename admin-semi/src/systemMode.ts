import { apiGet } from './api';
import type { NavGroup } from './config/adminMenu';

/**
 * 系统模式（我方新增，原版无）：config 表 biz_code=systemMode，
 * 后台读/写走与 buyStyle 同构的接口：
 * - 后台读：GET /setting/config/systemModeSettings
 * - 后台写：POST /setting/config/saveSystemMode
 * - 前台公开读：GET /api/common/config/systemMode
 * 值 { mode: 'all' | 'giftcard' | 'mall' }，缺失/非法一律归一化为 'all'（默认全功能）。
 */

export type SystemMode = 'all' | 'giftcard' | 'mall';

export const SYSTEM_MODE_CACHE_KEY = 'admin_system_mode';

export function normalizeSystemMode(raw: unknown): SystemMode {
  const mode = typeof raw === 'string' ? raw : (raw as { mode?: unknown } | null)?.mode;
  return mode === 'giftcard' || mode === 'mall' ? mode : 'all';
}

export function getCachedSystemMode(): SystemMode {
  try {
    return normalizeSystemMode(localStorage.getItem(SYSTEM_MODE_CACHE_KEY));
  } catch {
    return 'all';
  }
}

export function cacheSystemMode(mode: SystemMode) {
  try {
    localStorage.setItem(SYSTEM_MODE_CACHE_KEY, mode);
  } catch {
    // localStorage 不可用时忽略，门控退回默认 all
  }
}

/** 拉取后台 systemMode 配置并缓存；失败时返回缓存值（默认 all，不隐藏任何菜单）。 */
export async function fetchSystemMode(): Promise<SystemMode> {
  try {
    const data = await apiGet<Record<string, any>>('/setting/config/systemModeSettings');
    const mode = normalizeSystemMode(data);
    cacheSystemMode(mode);
    return mode;
  } catch {
    return getCachedSystemMode();
  }
}

/**
 * 后台菜单门控（保守版，仅渲染层，不拦路由直达）：
 * - mode=mall（商城系统）：隐藏「营销 > 礼品卡」（giftCardManage）。
 * - mode=giftcard（礼品卡系统）：隐藏营销下除礼品卡外的项——
 *   「营销管理」三级项仅保留礼品卡（优惠券 couponManage、限时秒杀 seckillManage、
 *   拼团 grouponPromotionManage 等普通营销一并隐藏），营销概览/营销样式同收起。
 * - mode=all / 拉取失败：原样返回，不隐藏。
 */
export function filterNavGroupsBySystemMode(groups: NavGroup[], mode: SystemMode): NavGroup[] {
  if (mode === 'all') return groups;
  return groups.map((group) => {
    if (group.key !== 'promotion') return group;
    if (mode === 'mall') {
      return {
        ...group,
        items: group.items.map((item) => ({
          ...item,
          children: item.children?.filter((child) => child.key !== 'giftCardManage'),
        })),
      };
    }
    // mode === 'giftcard'：营销组只保留礼品卡
    return {
      ...group,
      items: group.items
        .filter((item) => item.key === 'promotion1')
        .map((item) => ({
          ...item,
          children: item.children?.filter((child) => child.key === 'giftCardManage'),
        })),
    };
  });
}
