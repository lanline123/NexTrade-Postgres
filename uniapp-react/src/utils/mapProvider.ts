import { useConfigStore } from "@/store/config";

/**
 * 地图服务商判定（自旧 utils/mapProvider.ts 等价移植）。
 * 旧逻辑：小程序恒原生；H5/App 当 config.mapProvider !== "tianditu"
 * 或 tiandituJsTk 未配置时回落原生 <map>。
 * 供 TigMap 组件与消费页共用。React 版直接读 Zustand getState()，
 * 与旧版读 Pinia store 等价。
 */
export const isNativeMapProvider = (): boolean => {
    // 小程序端恒为原生地图（对应旧 #ifdef MP 恒 true）
    if (process.env.TARO_ENV !== "h5") return true;
    const { mapProvider, tiandituJsTk } = useConfigStore.getState();
    return mapProvider !== "tianditu" || !tiandituJsTk;
};
