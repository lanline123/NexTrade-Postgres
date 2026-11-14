/**
 * store 统一导出（补齐旧版 src/store/index.ts 的 barrel 角色）。
 * 此前 12 个 Zustand store 只能逐文件直引；此处集中再导出，
 * 既有逐文件 import 不受影响，新代码可从 "@/store" 统一引入。
 */
export { useAppVersionStore } from "./appVersion";
export { useCartStore } from "./cart";
export { usecatnavStore } from "./catnav";
export { useConfigStore } from "./config";
export { useCurrencyStore } from "./currency";
export { useI18nStore, syncFrameworkLocale, willFrameworkLocaleChange, clearLocaleSyncGuard } from "./i18n";
export { useImageSearchStore } from "./imageSearch";
export { useRegionStore } from "./region";
export { useSearchDecorateStore } from "./searchDecorate";
export { useTabbarStore, useTabbarDerived } from "./tabbar";
export { useThemeStore } from "./theme";
export { useUserStore } from "./user";
