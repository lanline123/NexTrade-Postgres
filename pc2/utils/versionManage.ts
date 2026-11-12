// 跨境独有：后台 version/translationVersion 变更时自动清理语言缓存并重拉翻译与基础配置
// 调用处（store/common.ts loadBaseConfig）已用 isOverseas() 守卫
import { useCommonStore } from "~/store/common";
import { useI18nStore } from "~/store/i18n";

export default async function compareVersion() {
    const commonStore = useCommonStore();

    if (!commonStore.version || !commonStore.translationVersion) return;

    const i18nInit = async () => {
        const i18nStore = useI18nStore();
        i18nStore.clearCache();
        await i18nStore.init();
    };

    const cacheVersion = localStorage.getItem("pcVersion") || "";

    const cacheTranslationVersion = localStorage.getItem("pcTranslationVersion") || 1;

    let needReloadConfig = false;

    if (cacheVersion !== commonStore.version) {
        await i18nInit();
        needReloadConfig = true;
        localStorage.setItem("pcVersion", commonStore.version);
    }

    if (Number(cacheTranslationVersion) !== commonStore.translationVersion) {
        if (!needReloadConfig) {
            await i18nInit();
        }
        localStorage.setItem("pcTranslationVersion", commonStore.translationVersion.toString());
    }

    if (needReloadConfig) {
        const route = useRoute();
        commonStore.loadBaseConfig({ previewId: route.query.previewId && Number(route.query.previewId) > 0 ? route.query.previewId : undefined });
        commonStore.loadThemeSettings();
    }
}
