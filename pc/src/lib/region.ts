import { apiGet } from '@/lib/client-api';

/**
 * Vue 基准：api/region.ts、components/region/src/SelectRegion.vue
 *
 * 真实接口（2026-11 实测 battery.recy.cc）：
 * - GET  sys/region/getProvinceList                       → 34 个省级节点（level=2，parentId=1）
 * - GET  sys/region/getRegionListByRegionId?regionId=      → 下一级节点数组
 * - GET  sys/region/getRegion?regionIds=1,310000,310100,310120 → 按层分组的数组（[国家], [省], [市], [区], []）
 *
 * 根节点 regionId = 1（中国），所以地址 regionIds 首项恒为 1。
 */
export const ROOT_REGION_ID = 1;

export interface RegionNode {
    regionId: number;
    regionName: string;
    parentId?: number;
    level?: number;
    firstWord?: string;
    isHot?: number;
}

export async function fetchProvinces(): Promise<RegionNode[]> {
    const list = await apiGet<RegionNode[]>('sys/region/getProvinceList');
    return Array.isArray(list) ? list : [];
}

export async function fetchRegionChildren(regionId: number): Promise<RegionNode[]> {
    const list = await apiGet<RegionNode[]>('sys/region/getRegionListByRegionId', { regionId: Number(regionId) });
    return Array.isArray(list) ? list : [];
}

/** 按 regionIds 还原完整层级（含国家层）。 */
export async function fetchRegionPaths(regionIds: number[]): Promise<RegionNode[][]> {
    if (!regionIds.length) return [];
    const groups = await apiGet<RegionNode[][]>('sys/region/getRegion', { regionIds: regionIds.join(',') });
    return Array.isArray(groups) ? groups.filter((group) => Array.isArray(group) && group.length > 0) : [];
}

/** "中国 上海市 上海市 奉贤区" → 用于展示。 */
export function regionLabel(names?: string[] | null, fallback?: string | null): string {
    const text = (names ?? []).filter(Boolean).join(' ');
    return text || fallback || '';
}
