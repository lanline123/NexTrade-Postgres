/**
 * 顶栏地区种子：RootLayout 在服务端按「手动选择 Cookie → 来访 IP 定位」解析，
 * 经 React Context 注入 PcHeaderTopBar 的初始状态，保证 SSR 首屏即显示真实定位，
 * 不再先闪装修配置里的默认文案、也不等客户端加载后再跳变。
 */

export interface RegionSeedProvince {
  regionId: number | string;
  regionName: string;
}

export interface RegionSeed {
  /** 解析出的地区 id（省份 id；IP 定位也可能返回城市级 id） */
  regionId: string;
  /** 与 regionId 对应的展示名（已按省份列表口径精简）；找不到时为 null */
  regionName: string | null;
  /** 省份列表（后台 province_list 已精简名），供顶栏下拉与按 id 查名 */
  provinces: RegionSeedProvince[];
}

/** 省份名精简：与后端 getProvinceList 的链式 replace 同口径（先省、再市、最后自治区）。 */
export function shortenProvinceName(name: string): string {
  return name.replace(/省/g, '').replace(/市/g, '').replace(/自治区/g, '');
}

/** 是否公网 IP（与后端 lookup 的内网过滤同口径：内网/回环/链路本地不做定位）。 */
export function isPublicIp(ip: string | null | undefined): boolean {
  if (!ip) return false;
  const value = ip.trim();
  if (!value || value === 'unknown') return false;
  if (value.includes(':')) {
    const lower = value.toLowerCase();
    if (lower === '::1' || lower === '::') return false;
    if (lower.startsWith('fe8') || lower.startsWith('fe9') || lower.startsWith('fea') || lower.startsWith('feb')) return false;
    if (lower.startsWith('fc') || lower.startsWith('fd')) return false;
    const mapped = lower.match(/::ffff:(\d+\.\d+\.\d+\.\d+)$/);
    if (mapped) return isPublicIp(mapped[1]);
    return true;
  }
  const parts = value.split('.').map((part) => Number(part));
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) return false;
  const [a, b] = parts;
  if (a === 10 || a === 127 || a === 0) return false;
  if (a === 172 && b >= 16 && b <= 31) return false;
  if (a === 192 && b === 168) return false;
  if (a === 169 && b === 254) return false;
  if (a === 100 && b >= 64 && b <= 127) return false;
  return true;
}
