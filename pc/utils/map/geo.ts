const EARTH_RADIUS_METERS = 6378137;
const DEG_TO_RAD = Math.PI / 180;

/** 国标行政区码的国家前缀，天地图返回 9 位（156 + 6 位 adcode） */
const CHINA_GB_PREFIX = "156";

/**
 * 两点球面距离
 * @description 天地图 LocalSearch 的结果不带 distance，周边搜索排序需要自算
 * @param from - 起点 [经度, 纬度]
 * @param to - 终点 [经度, 纬度]
 * @returns 距离（米）
 */
export const distanceBetween = (from: [number, number], to: [number, number]): number => {
    const [lng1, lat1] = from;
    const [lng2, lat2] = to;
    const dLat = (lat2 - lat1) * DEG_TO_RAD;
    const dLng = (lng2 - lng1) * DEG_TO_RAD;
    const a =
        Math.sin(dLat / 2) * Math.sin(dLat / 2) +
        Math.cos(lat1 * DEG_TO_RAD) * Math.cos(lat2 * DEG_TO_RAD) * Math.sin(dLng / 2) * Math.sin(dLng / 2);
    return 2 * EARTH_RADIUS_METERS * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
};

/**
 * 归一化行政区码
 * @description 天地图返回 9 位国标码（156 + 6 位），高德返回 6 位 adcode。
 * 统一成 6 位，否则 getRegionPath 反查省市区会拿不到结果
 * @param raw - 原始行政区码
 * @returns 6 位 adcode，无法识别时返回空串
 */
export const normalizeAdcode = (raw?: string | number | null): string => {
    const code = String(raw ?? "").trim();
    if (!code) return "";
    if (code.length === 6) return code;
    if (code.length === 9 && code.startsWith(CHINA_GB_PREFIX)) return code.slice(CHINA_GB_PREFIX.length);
    return code;
};

/**
 * 解析 "经度,纬度" 字符串
 * @param raw - 形如 "116.397428,39.90923" 的坐标串
 * @returns 合法时返回 [经度, 纬度]，否则返回 null
 */
export const parseLngLat = (raw?: string | null): [number, number] | null => {
    const [lng, lat] = String(raw ?? "")
        .split(",")
        .map(Number);
    if (!Number.isFinite(lng) || !Number.isFinite(lat)) return null;
    return [lng, lat];
};
