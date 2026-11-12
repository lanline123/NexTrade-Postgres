/** 地图服务商标识，与 types/map 中的 MapProvider 取值一致 */
type MapProvider = "amap" | "tianditu";

export const MAP_PROVIDER_AMAP: MapProvider = "amap";
export const MAP_PROVIDER_TIANDITU: MapProvider = "tianditu";

/**
 * 解析地图服务商
 * @description 与后端同口径：空串、空白、未知取值一律按高德处理，不抛错。
 * 存量部署未执行增量 SQL 时该字段不下发（undefined），同样落到高德分支
 * @param raw - 后端 initConfigSettings 下发的 mapProvider 原始值
 * @returns 归一化后的服务商标识
 */
export const resolveMapProvider = (raw?: string | null): MapProvider => {
    return String(raw ?? "").trim() === MAP_PROVIDER_TIANDITU ? MAP_PROVIDER_TIANDITU : MAP_PROVIDER_AMAP;
};

/**
 * 是否为天地图
 * @param raw - 后端下发的 mapProvider 原始值
 */
export const isTiandituProvider = (raw?: string | null): boolean => {
    return resolveMapProvider(raw) === MAP_PROVIDER_TIANDITU;
};
