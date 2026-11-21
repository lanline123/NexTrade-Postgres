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
  /** 地区路径派生：所属省份名（已精简），供顶栏中文口径「只显示省份」 */
  provinceName?: string | null;
  /** 地区路径派生：所属国家名（地区表 level=1 行的 regionName，如「中国」） */
  countryName?: string | null;
  /** 地区路径派生：所属国家标准代码（level=1 行的 standardCode，如「CN」） */
  countryCode?: string | null;
}

/** getRegionPath 返回行（region_json 形状，只取派生用到的字段）。 */
export interface RegionPathRow {
  regionId?: number | string;
  regionName?: string;
  level?: number | string | null;
  standardCode?: string | null;
}

/** 顶栏定位文案的路径派生结果（省份 / 国家 / 国家代码）。 */
export interface RegionPathInfo {
  provinceName: string | null;
  countryName: string | null;
  countryCode: string | null;
}

export const EMPTY_REGION_PATH: RegionPathInfo = { provinceName: null, countryName: null, countryCode: null };

/**
 * 由 getRegionPath 的根→叶路径派生顶栏展示要素：
 * 国家 = level=1 行（路径回溯到默认国家即停，中国树首行即国家；
 * 外国地区走到根，首行同样是国家），省份 = level=2 行（名按省份列表口径精简）。
 */
export function deriveRegionPathInfo(rows: RegionPathRow[] | null | undefined): RegionPathInfo {
  const list = Array.isArray(rows) ? rows.filter((row) => row && typeof row === 'object') : [];
  if (!list.length) return { ...EMPTY_REGION_PATH };
  const country = list.find((row) => Number(row.level) === 1) ?? list[0];
  const province = list.find((row) => Number(row.level) === 2);
  const code = country?.standardCode ? String(country.standardCode).trim().toUpperCase() : '';
  return {
    provinceName: province?.regionName ? shortenProvinceName(String(province.regionName)) : null,
    countryName: country?.regionName ? String(country.regionName) : null,
    countryCode: code || null,
  };
}

/**
 * 非中文口径的国家英文名：数据自带 standardCode 派生，不自造翻译表。
 * 中国按用户口径固定「China」；其余国家用平台 Intl.DisplayNames（ICU 地区库）
 * 按代码取英文名；代码缺失/派生失败时回落地区表自带名（其中「中国」同样映射 China）。
 */
export function countryDisplayNameEn(info: RegionPathInfo): string | null {
  const code = info.countryCode ? String(info.countryCode).trim().toUpperCase() : '';
  if (code === 'CN') return 'China';
  if (/^[A-Z]{2}$/.test(code)) {
    try {
      const name = new Intl.DisplayNames(['en'], { type: 'region' }).of(code);
      if (name && name !== code) return name;
    } catch {
      /* ICU 不可用时走下方回落 */
    }
  }
  if (info.countryName === '中国') return 'China';
  return info.countryName || null;
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
