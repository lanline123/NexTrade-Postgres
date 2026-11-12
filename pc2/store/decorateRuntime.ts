import type { PcHeaderConfig } from "~/types/decorate/pcHeader";
import { decorateApi } from "~/api/decorate/pcHeader";
import { resolvePcHeaderConfigFromResult } from "~/utils/resolvePcHeaderConfig";

/** PC 搜索页装修配置（pcSearchResult 模块） */
export interface PcSearchModuleData {
    backgroundColor?: string;
    themeColor?: string;
    filterStyle?: number;
    filterTemplate?: number;
    showFilterPanel?: boolean;
    showPriceRange?: boolean;
    showShopTag?: boolean;
    showSales?: boolean;
    showCartButton?: boolean;
    goodsStyle?: number;
    goodsRadiusStyle?: number;
    buyStyle?: number;
    componentMarginTop?: number;
    componentMarginBottom?: number;
    [key: string]: unknown;
}

/**
 * 前台运行态装修状态（与首页装修 store 解耦）
 * 仅存放「布局层需要感知」的全局模块——当前为 PC 头部装修。
 * 全站统一加载一次（app.vue），按 previewId 缓存，支持页面级覆盖。
 */
export const useDecorateRuntimeStore = defineStore("decorateRuntime", () => {
    // 全局头部配置（来源：home/home/pcHeader），全站通用，按 previewId 缓存
    const globalPcHeaderConfig = ref<PcHeaderConfig | null>(null);
    const globalPcHeaderPreviewId = ref<string>("");
    const globalPcHeaderLoading = ref<Promise<void> | null>(null);
    const globalPcHeaderResolved = ref(false);
    // 在途请求的目标 previewId：用于并发去重与丢弃过期结果（连点切换风格）
    const globalPcHeaderPendingPreviewId = ref<string | null>(null);

    // 页面级头部覆盖（来源：首页装修返回的 pageModule.pcHeader）
    const pagePcHeaderOverride = ref<PcHeaderConfig | null>(null);
    const pagePcHeaderRouteKey = ref("");

    // PC 搜索页装修配置（来源：decoration/decorate pcSearch），供搜索页读取
    const pcSearchConfig = ref<PcSearchModuleData | null>(null);

    /**
     * 风格切换挂起态（演示站 ChooseStyle）。
     * 为 true 时首页装修数据先不提交，等新头部配置一起在同一帧落地，
     * 避免出现「宽度先变、模块后变」的中间态闪烁。
     */
    const styleSwitching = ref(false);
    function setStyleSwitching(value: boolean) {
        styleSwitching.value = value;
    }

    /** 设置 PC 搜索页装修配置 */
    function setPcSearchConfig(config: PcSearchModuleData | null) {
        pcSearchConfig.value = config;
    }

    // 对外统一读取口：优先页面覆盖，否则用全局配置
    const pcHeaderConfig = computed<PcHeaderConfig | null>(() => {
        return pagePcHeaderOverride.value ?? globalPcHeaderConfig.value;
    });
    // 全局头部是否已解析完成（未完成时头部占位防闪动）
    const pcHeaderReady = computed<boolean>(() => globalPcHeaderResolved.value);

    /**
     * 首页是否新版装修风格（全站宽度体系分流信号）。
     * 后端 pcHeader 取自 PC 首页 pageModule.pcHeader，旧版首页无此配置块 → 返回空，
     * 故「已解析且拿到头部配置」即等价于首页新版装修；据此让全站 header 与 .container
     * 容器统一切换宽度（新版 1264–1600 流式 / 旧版固定 1190），并与 header 新旧版渲染同源对齐。
     */
    const isHomeNewStyle = computed<boolean>(() => pcHeaderReady.value && !!pcHeaderConfig.value);

    function normalizePreviewId(previewId?: string | number) {
        if (!previewId) return "";
        const v = String(previewId);
        return v === "0" ? "" : v;
    }

    /** 设置页面级头部覆盖（首页装修返回的 pageModule.pcHeader） */
    function setPcHeaderConfig(config: PcHeaderConfig | null, routeKey = "") {
        pagePcHeaderOverride.value = config;
        pagePcHeaderRouteKey.value = routeKey;
    }

    /**
     * 拉取并提交头部装修配置。
     * reload=true 表示 previewId 变化后的客户端重取：不清 resolved、不清旧配置，
     * 等新配置到位后一次性替换，避免头部与全站宽度中途回退到旧版再跳回来。
     */
    async function loadGlobalPcHeaderConfig(pid: string, reload: boolean): Promise<void> {
        if (!reload) globalPcHeaderResolved.value = false;
        globalPcHeaderPendingPreviewId.value = pid;

        const task = (async () => {
            const params = { previewId: pid ? pid : undefined };
            try {
                const headerResult = reload ? await decorateApi.getPcHeaderAsync(params) : await decorateApi.getPcHeader(params);
                if (globalPcHeaderPendingPreviewId.value !== pid) return;
                globalPcHeaderConfig.value = resolvePcHeaderConfigFromResult(headerResult);
                globalPcHeaderPreviewId.value = pid;
            } catch {
                if (globalPcHeaderPendingPreviewId.value !== pid) return;
                globalPcHeaderConfig.value = null;
                globalPcHeaderPreviewId.value = pid;
            } finally {
                // 已有更新的切换在途时不收尾，交给最后一次请求统一提交
                if (globalPcHeaderPendingPreviewId.value === pid) {
                    globalPcHeaderResolved.value = true;
                    globalPcHeaderLoading.value = null;
                    globalPcHeaderPendingPreviewId.value = null;
                }
            }
        })();

        globalPcHeaderLoading.value = task;
        return await task;
    }

    /** 全站统一加载头部装修配置（缓存 + 并发去重 + ready 态） */
    async function ensureGlobalPcHeaderConfig(previewId?: string | number): Promise<void> {
        const pid = normalizePreviewId(previewId);

        // 已加载且 previewId 未变化
        if (globalPcHeaderResolved.value && globalPcHeaderPreviewId.value === pid) return;

        // 并发去重：仅当在途请求就是同一个 previewId
        if (globalPcHeaderLoading.value && globalPcHeaderPendingPreviewId.value === pid) return await globalPcHeaderLoading.value;

        return await loadGlobalPcHeaderConfig(pid, false);
    }

    /**
     * previewId 变化后重新加载头部装修配置（演示站切风格 / 手动改 query）。
     * 头部新旧版与全站 --pc-home-* 宽度体系都由这份配置派生，不重取会残留上一套风格的容器宽度。
     */
    async function reloadGlobalPcHeaderConfig(previewId?: string | number): Promise<void> {
        const pid = normalizePreviewId(previewId);

        if (globalPcHeaderLoading.value && globalPcHeaderPendingPreviewId.value === pid) return await globalPcHeaderLoading.value;
        if (globalPcHeaderResolved.value && globalPcHeaderPreviewId.value === pid && !globalPcHeaderLoading.value) return;

        return await loadGlobalPcHeaderConfig(pid, true);
    }

    /**
     * 预取头部装修配置但不提交，返回提交函数。
     * 风格切换用：调用方拿到首页装修数据后再执行提交函数，让头部、全站宽度与首页模块同帧生效。
     */
    async function prepareGlobalPcHeaderConfig(previewId?: string | number): Promise<() => void> {
        const pid = normalizePreviewId(previewId);
        let config: PcHeaderConfig | null = null;
        try {
            config = resolvePcHeaderConfigFromResult(await decorateApi.getPcHeaderAsync({ previewId: pid ? pid : undefined }));
        } catch {
            config = null;
        }
        return () => {
            globalPcHeaderConfig.value = config;
            globalPcHeaderPreviewId.value = pid;
            globalPcHeaderPendingPreviewId.value = null;
            globalPcHeaderLoading.value = null;
            globalPcHeaderResolved.value = true;
        };
    }

    // 路由切换时调用：只清页面级覆盖，保留全局缓存，避免重复请求
    function reset(routeKey = "") {
        if (routeKey && pagePcHeaderRouteKey.value === routeKey) {
            return;
        }
        pagePcHeaderOverride.value = null;
        pagePcHeaderRouteKey.value = "";
    }

    return {
        pcHeaderConfig,
        pcHeaderReady,
        isHomeNewStyle,
        setPcHeaderConfig,
        ensureGlobalPcHeaderConfig,
        reloadGlobalPcHeaderConfig,
        prepareGlobalPcHeaderConfig,
        styleSwitching,
        setStyleSwitching,
        pcSearchConfig,
        setPcSearchConfig,
        reset
    };
});
