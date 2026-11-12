import type { ModuleListType } from "@/types/api.d";

export const useHomeStore = defineStore("home", () => {
    const moduleList = ref<ModuleListType[]>([]);
    const moduleLoaded = ref(false);
    /** 新一轮装修数据是否已取到（与「是否已提交到 store」解耦，供风格切换判断何时整体落地） */
    const moduleReady = ref(false);
    const decorateId = ref(0);
    const headKeywords = ref("");
    const headDescription = ref("");
    const previewId = ref(0);
    /** 页面级配置（后端透传 pageModule，含 homeStyleVersion 用于新旧分流） */
    const pageModule = ref<Record<string, any> | null>(null);

    return {
        moduleList,
        moduleLoaded,
        moduleReady,
        decorateId,
        headKeywords,
        headDescription,
        previewId,
        pageModule
    };
});
