import { apiGet } from '../api';

/**
 * 后台地图共享基础（高德 / 天地图双分支）：
 * 地图设置读取、两家 JS SDK 加载、GCJ-02 <-> WGS84 坐标转换。
 * 由 DeliveryRadiusMap（配送范围圈）与 MapLocationPicker（门店/自提点地图选点）共用，
 * 全后台只此一套加载与转换口径，不另起平行实现。
 */

export interface MapSettings {
  mapProvider?: string;
  amapJsKey?: string;
  amapJsSecret?: string;
  tiandituJsTk?: string;
}

export type LngLat = [number, number];

export function hasUsableKey(value?: string) {
  const key = String(value || '').trim();
  return Boolean(key && key !== '0');
}

export function isValidLngLat(value?: LngLat | null): value is LngLat {
  return Boolean(
    value &&
      Number.isFinite(value[0]) &&
      Number.isFinite(value[1]) &&
      (value[0] !== 0 || value[1] !== 0),
  );
}

export function fetchMapSettings(): Promise<MapSettings | null> {
  return apiGet<MapSettings>('/setting/config/mapSettings').catch(() => null);
}

/**
 * 加载高德 JS API。插件列表取两组件所需并集（CircleEditor 供配送圈编辑、
 * Geocoder/PlaceSearch 供选点搜索与逆地理），任一组件先加载都不会缺插件；
 * 若 AMap 已被先前页面加载，直接复用（与原 DeliveryRadiusMap 行为一致）。
 */
export function loadAmapSdk(key: string, secret: string): Promise<any> {
  return new Promise<any>((resolve, reject) => {
    const existing = (window as any).AMap;
    if (existing) {
      resolve(existing);
      return;
    }
    if (secret) {
      (window as any)._AMapSecurityConfig = { securityJsCode: secret };
    }
    const script = document.createElement('script');
    script.src = `https://webapi.amap.com/maps?v=2.0&key=${encodeURIComponent(key)}&plugin=AMap.CircleEditor,AMap.Geocoder,AMap.PlaceSearch`;
    script.async = true;
    script.onload = () => resolve((window as any).AMap);
    script.onerror = () => reject(new Error('地图加载失败'));
    document.head.appendChild(script);
  });
}

/** 加载天地图 JS API 4.0（与原 DeliveryRadiusMap 行为一致）。 */
export function loadTiandituSdk(tk: string): Promise<any> {
  return new Promise<any>((resolve, reject) => {
    const existing = (window as any).T;
    if (existing?.Map) {
      resolve(existing);
      return;
    }
    const script = document.createElement('script');
    script.src = `https://api.tianditu.gov.cn/api?v=4.0&tk=${encodeURIComponent(tk)}`;
    script.async = true;
    script.onload = () => {
      const T = (window as any).T;
      if (T?.Map) resolve(T);
      else reject(new Error('地图加载失败'));
    };
    script.onerror = () => reject(new Error('地图加载失败'));
    document.head.appendChild(script);
  });
}

// ——— GCJ-02 <-> WGS84（天地图 CGCS2000 按 WGS84 处理）———
// 门店经纬度按高德口径存 GCJ-02；天地图分支进出地图前转换。
// admin 无 gcoord 依赖，此处内嵌标准算法（Krasovsky 椭球参数 + 迭代反解，
// 与 H5 侧 gcoord 的 GCJ02<->WGS84 口径一致，误差米级，配送圈展示足够）。
const GCJ_A = 6378245.0;
const GCJ_EE = 0.00669342162296594323;

function outOfChina(lng: number, lat: number) {
  return lng < 72.004 || lng > 137.8347 || lat < 0.8293 || lat > 55.8271;
}

function transformLat(x: number, y: number) {
  let ret = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * Math.sqrt(Math.abs(x));
  ret += ((20.0 * Math.sin(6.0 * x * Math.PI) + 20.0 * Math.sin(2.0 * x * Math.PI)) * 2.0) / 3.0;
  ret += ((20.0 * Math.sin(y * Math.PI) + 40.0 * Math.sin((y / 3.0) * Math.PI)) * 2.0) / 3.0;
  ret += ((160.0 * Math.sin((y / 12.0) * Math.PI) + 320 * Math.sin((y * Math.PI) / 30.0)) * 2.0) / 3.0;
  return ret;
}

function transformLng(x: number, y: number) {
  let ret = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * Math.sqrt(Math.abs(x));
  ret += ((20.0 * Math.sin(6.0 * x * Math.PI) + 20.0 * Math.sin(2.0 * x * Math.PI)) * 2.0) / 3.0;
  ret += ((20.0 * Math.sin(x * Math.PI) + 40.0 * Math.sin((x / 3.0) * Math.PI)) * 2.0) / 3.0;
  ret += ((150.0 * Math.sin((x / 12.0) * Math.PI) + 300.0 * Math.sin((x / 30.0) * Math.PI)) * 2.0) / 3.0;
  return ret;
}

export function wgs84ToGcj02(lng: number, lat: number): LngLat {
  if (outOfChina(lng, lat)) return [lng, lat];
  let dLat = transformLat(lng - 105.0, lat - 35.0);
  let dLng = transformLng(lng - 105.0, lat - 35.0);
  const radLat = (lat / 180.0) * Math.PI;
  let magic = Math.sin(radLat);
  magic = 1 - GCJ_EE * magic * magic;
  const sqrtMagic = Math.sqrt(magic);
  dLat = (dLat * 180.0) / (((GCJ_A * (1 - GCJ_EE)) / (magic * sqrtMagic)) * Math.PI);
  dLng = (dLng * 180.0) / ((GCJ_A / sqrtMagic) * Math.cos(radLat) * Math.PI);
  return [lng + dLng, lat + dLat];
}

export function gcj02ToWgs84(lng: number, lat: number): LngLat {
  if (outOfChina(lng, lat)) return [lng, lat];
  // 迭代反解：以 GCJ 点为初值，两轮逼近把正向变换残差压到米级以内。
  let wgs: LngLat = [lng, lat];
  for (let i = 0; i < 2; i += 1) {
    const gcj = wgs84ToGcj02(wgs[0], wgs[1]);
    wgs = [lng - (gcj[0] - wgs[0]), lat - (gcj[1] - wgs[1])];
  }
  return wgs;
}
