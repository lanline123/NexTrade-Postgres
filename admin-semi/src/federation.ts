import { apiGet } from './api';
import type { NavGroup } from './config/adminMenu';

/**
 * 万象互联（我方新增功能，原版无；原名「平台联邦」）：多实例经 API 联邦（hub-federation-design §2/§6）。
 *
 * 设置走与 systemMode 同构的常驻接口（不受总开关门控，关时也能重新打开）：
 * - 后台读：GET /setting/config/federationSettings
 * - 后台写：POST /setting/config/saveFederationSettings
 * 组织域接口挂 /hub/*，总开关关时后端 404。
 *
 * 门控口径（照 systemMode 模式，仅渲染层隐藏，不拦直达）：
 * - enabled=false → 一级菜单「万象互联」整体不出现，「组织」组下「商品流通」栏一并不出现；
 * - enabled=true 时按 capabilities/modules 摘子页（见 filterNavGroupsByFederation）。
 */

export interface FederationCapabilities {
  supplyOut: boolean;
  supplyIn: boolean;
  federation: boolean;
}

export interface FederationModules {
  product: boolean;
  member: boolean;
  order: boolean;
}

export interface FederationSettings {
  enabled: boolean;
  capabilities: FederationCapabilities;
  modules: FederationModules;
  instanceUuid: string;
  instanceName: string;
  instanceBaseUrl: string;
}

export const FEDERATION_CACHE_KEY = 'admin_federation';

export const DEFAULT_FEDERATION_SETTINGS: FederationSettings = {
  enabled: false,
  capabilities: { supplyOut: false, supplyIn: false, federation: false },
  modules: { product: false, member: false, order: false },
  instanceUuid: '',
  instanceName: '',
  instanceBaseUrl: '',
};

function asBool(value: unknown): boolean {
  return value === true || value === 1 || value === '1';
}

function asRecord(value: unknown): Record<string, any> {
  return value && typeof value === 'object' ? (value as Record<string, any>) : {};
}

/** 归一化联邦设置：缺失/非法一律回落全关默认（enabled=false 等）。 */
export function normalizeFederationSettings(raw: unknown): FederationSettings {
  const data = asRecord(raw);
  const capabilities = asRecord(data.capabilities);
  const modules = asRecord(data.modules);
  return {
    enabled: asBool(data.enabled),
    capabilities: {
      supplyOut: asBool(capabilities.supplyOut),
      supplyIn: asBool(capabilities.supplyIn),
      federation: asBool(capabilities.federation),
    },
    modules: {
      product: asBool(modules.product),
      member: asBool(modules.member),
      order: asBool(modules.order),
    },
    instanceUuid: typeof data.instanceUuid === 'string' ? data.instanceUuid : '',
    instanceName: typeof data.instanceName === 'string' ? data.instanceName : '',
    instanceBaseUrl: typeof data.instanceBaseUrl === 'string' ? data.instanceBaseUrl : '',
  };
}

export function getCachedFederationSettings(): FederationSettings {
  try {
    const raw = localStorage.getItem(FEDERATION_CACHE_KEY);
    return raw ? normalizeFederationSettings(JSON.parse(raw)) : { ...DEFAULT_FEDERATION_SETTINGS };
  } catch {
    return { ...DEFAULT_FEDERATION_SETTINGS };
  }
}

export function cacheFederationSettings(settings: FederationSettings) {
  try {
    localStorage.setItem(FEDERATION_CACHE_KEY, JSON.stringify(normalizeFederationSettings(settings)));
  } catch {
    // localStorage 不可用时忽略，门控退回默认全关
  }
}

/** 拉取后台联邦设置并缓存；失败时返回缓存值（默认全关，万象互联菜单不出现）。 */
export async function fetchFederationSettings(): Promise<FederationSettings> {
  try {
    const data = await apiGet<Record<string, any>>('/setting/config/federationSettings');
    const settings = normalizeFederationSettings(data);
    cacheFederationSettings(settings);
    return settings;
  } catch {
    return getCachedFederationSettings();
  }
}

/**
 * 后台菜单门控（仅渲染层，不拦路由直达）。落位说明：联邦六页自成一级组
 * 「万象互联」（key='federation'，2026-11-21 由「组织」组下二级栏位提为一级）；
 * 「商品流通」（organize7）仍留「组织」组下，但同受本门控：
 * - enabled=false → 「万象互联」整组不出现，organize7 一并摘掉（组织组其余原版栏位不受影响）；
 * - enabled=true：
 *   - capabilities.supplyOut 与 federation 皆关 → 摘 hubCredentials；
 *   - capabilities.federation 关 → 摘 hubLinks / hubPairing / myLinks；
 *   - modules.product 关 → 摘 hubAggregation 与 feedKeyManage；
 *   - 子页摘空 → 整组/整栏摘掉。
 */
const FEDERATION_GROUP_KEY = 'federation';
const CIRCULATION_ITEM_KEY = 'organize7';

export function filterNavGroupsByFederation(groups: NavGroup[], settings: FederationSettings): NavGroup[] {
  const removeCredentials = !settings.capabilities.supplyOut && !settings.capabilities.federation;
  const removeFederation = !settings.capabilities.federation;
  const removeProduct = !settings.modules.product;
  const shouldRemove = (key: string): boolean => {
    if (!settings.enabled) return true;
    if (key === 'hubCredentials') return removeCredentials;
    if (key === 'hubLinks' || key === 'hubPairing' || key === 'myLinks') return removeFederation;
    if (key === 'hubAggregation' || key === 'feedKeyManage') return removeProduct;
    return false;
  };
  return groups.flatMap((group) => {
    if (group.key === FEDERATION_GROUP_KEY) {
      if (!settings.enabled) return [];
      const items = group.items.filter((item) => !shouldRemove(item.key));
      return items.length > 0 ? [{ ...group, items }] : [];
    }
    if (group.key === 'organize') {
      return [
        {
          ...group,
          items: group.items
            .filter((item) => item.key !== CIRCULATION_ITEM_KEY || settings.enabled)
            .map((item) =>
              item.key === CIRCULATION_ITEM_KEY
                ? { ...item, children: item.children?.filter((child) => !shouldRemove(child.key)) }
                : item,
            )
            .filter(
              (item) =>
                item.key !== CIRCULATION_ITEM_KEY || (item.children && item.children.length > 0),
            ),
        },
      ];
    }
    return [group];
  });
}

/** 凭证 scopes 位定义（复选框 → 位集数值），HubCredentials 页共用。 */
export const HUB_SCOPES: Array<{ bit: number; key: string; label: string }> = [
  { bit: 1, key: 'meta.read', label: 'meta.read 元信息' },
  { bit: 2, key: 'product.read', label: 'product.read 商品读取' },
  { bit: 4, key: 'product.push', label: 'product.push 商品推送' },
  { bit: 8, key: 'order.report', label: 'order.report 订单上报' },
  { bit: 16, key: 'order.read', label: 'order.read 订单读取' },
  { bit: 32, key: 'member.sync', label: 'member.sync 会员同步' },
  { bit: 64, key: 'member.read', label: 'member.read 会员读取' },
  { bit: 128, key: 'member.auth', label: 'member.auth 会员鉴权' },
  { bit: 256, key: 'config.pull', label: 'config.pull 配置拉取' },
  { bit: 512, key: 'stats.report', label: 'stats.report 统计/心跳上报' },
  { bit: 2048, key: 'order.place', label: 'order.place 跨店下单' },
  { bit: 4096, key: 'wallet.post', label: 'wallet.post 钱包过账' },
];

/** 连接批准模块位集（1商品/2会员/4订单）。 */
export const HUB_MODULE_BITS: Array<{ bit: number; label: string }> = [
  { bit: 1, label: '商品' },
  { bit: 2, label: '会员' },
  { bit: 4, label: '订单' },
];

export function moduleNamesFromBits(bits: number): string[] {
  return HUB_MODULE_BITS.filter((item) => (bits & item.bit) === item.bit).map((item) => item.label);
}
