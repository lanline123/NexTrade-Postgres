/**
 * PC 登录页装修（home/home/pcLogin）读取与派生
 * - 登录页与注册页共用同一份配置，SSR 拉取，key 固定复用避免重复请求
 * - 接口结构：moduleList[].module 才是业务配置（与搜索页 pcSearchResult 一致）
 * - 对后端下发的 0/1、字符串做布尔/数值归一化
 */
import { computed } from "vue";
import { decorateApi } from "~/api/decorate/pcHeader";
import { getMobileNav } from "~/api/home/home";
import { isOverseas } from "~/utils/util";
import type { ApiPcLoginModuleConfig } from "~/types/decoration/decorationClientVo";

// 登录区最小高度：默认/下限 620，与装修滑块一致（经典样式-非跨境 不设最小高度，见 warpStyle）
const WARP_HEIGHT_MIN = 620;

// 后端装修下发常见为 0/1 或字符串，做布尔/数值归一化
const normalizeBoolLike = (val: unknown, defaultVal: boolean): boolean => {
    if (val === undefined || val === null || val === "") return defaultVal;
    if (val === false || val === 0 || val === "0" || val === "false") return false;
    if (val === true || val === 1 || val === "1" || val === "true") return true;
    return Boolean(val);
};
const normalizeNumberLike = (val: unknown, defaultVal: number): number => {
    if (val === undefined || val === null || val === "") return defaultVal;
    const num = Number(val);
    return Number.isFinite(num) ? num : defaultVal;
};

/**
 * 老客户装修兜底：新接口 pcLogin 无数据时，回退老接口 pcUserLogin，
 * 并按运行环境 isOverseas() 输出两套老样式默认值（下游 computed 无需改动）：
 * - 跨境=现代版(OverseasLogin)：头部 70 + 阴影 + logo 宽 140 高自适应 + 白底
 * - 其余=O2O 经典版(BaseLogin)：头部 90 + 1px 底边框(无阴影) + logo 宽 190 高自适应 + 老背景
 * 注：logoHeight=0 会被 LoginHeader 解释为高度 auto（仅固定宽度、按比例）。
 */
const getLegacyLoginDecorate = async (): Promise<ApiPcLoginModuleConfig> => {
    if (isOverseas()) {
        return { loginMode: 2, headerHeight: 70, headerShadow: true, logoWidth: 140, logoHeight: 0 };
    }

    let backgroundPic: ApiPcLoginModuleConfig["backgroundPic"];
    try {
        const legacy = await getMobileNav({ decorateSn: "pcUserLogin" });
        backgroundPic = legacy?.data?.backgroundPic;
    } catch (error) {
        // 老接口失败也返回样式兜底（仅背景为空），保证页面可用
    }

    return { loginMode: 1, headerHeight: 90, headerShadow: false, logoWidth: 190, logoHeight: 0, backgroundPic };
};

export const usePcLoginDecorate = async () => {
    const route = useRoute();
    const previewId = computed(() => Number(route.query.previewId) || 0);

    // SSR 拉取 PC 登录页装修配置，首屏直出，避免经典/现代切换闪动
    const { data: loginDecorate } = await useAsyncData(
        "pc-login-decorate",
        async (): Promise<ApiPcLoginModuleConfig | null> => {
            try {
                const result = await decorateApi.getPcLogin(previewId.value > 0 ? { previewId: previewId.value } : undefined);
                const moduleList = Array.isArray(result?.moduleList) ? result.moduleList : [];
                const item = moduleList.find((m) => m.type === "pcLogin") ?? moduleList[0];
                // 新客户：命中新版 pcLogin 装修则原样返回
                if (item?.module && Object.keys(item.module).length > 0) return item.module;

                // 老客户兜底：预览态不回退，避免污染装修预览
                if (previewId.value > 0) return null;

                return await getLegacyLoginDecorate();
            } catch (error) {
                // 新接口异常也走老装修兜底，保证登录页可用
                if (previewId.value > 0) return null;
                return await getLegacyLoginDecorate();
            }
        },
        { watch: [previewId], default: () => null }
    );

    // 登录模式：1=经典(BaseLogin) / 2=现代(OverseasLogin)，默认现代，对齐后端兜底
    const loginMode = computed(() => (Number(loginDecorate.value?.loginMode) === 1 ? 1 : 2));
    // 头部高度/阴影（默认对齐前台 CommonLoginHeader 70px + 阴影）
    const headerHeight = computed(() => normalizeNumberLike(loginDecorate.value?.headerHeight, 70));
    const headerShadow = computed(() => normalizeBoolLike(loginDecorate.value?.headerShadow, true));
    // Logo 宽高（默认 140×40，配合 object-fit: contain 防止变形）
    const logoWidth = computed(() => normalizeNumberLike(loginDecorate.value?.logoWidth, 140));
    const logoHeight = computed(() => normalizeNumberLike(loginDecorate.value?.logoHeight, 40));
    // 登录区最小高度：默认/下限 620，与装修滑块一致
    const warpHeight = computed(() => Math.max(WARP_HEIGHT_MIN, normalizeNumberLike(loginDecorate.value?.warpHeight, WARP_HEIGHT_MIN)));
    // 头部欢迎语与登录/注册互链是否展示（默认展示）
    const showWelcome = computed(() => normalizeBoolLike(loginDecorate.value?.showWelcome, true));

    // 登录区背景：背景图居中不平铺 + 背景色（对齐装修预览与前台 .login-warp）
    const warpStyle = computed(() => {
        const picUrl = loginDecorate.value?.backgroundPic?.picUrl;
        const style: Record<string, string> = {
            backgroundColor: loginDecorate.value?.backgroundColor || "#fff",
            backgroundImage: picUrl ? `url(${picUrl})` : "none"
        };
        // 经典样式(非跨境)不设最小高度，让内容自然撑开；其余作为最小高度，下限 620
        const isClassicDomestic = Number(loginDecorate.value?.loginMode) === 1 && !isOverseas();
        if (!isClassicDomestic) {
            style.minHeight = `${warpHeight.value}px`;
        }
        return style;
    });

    return {
        loginDecorate,
        loginMode,
        headerHeight,
        headerShadow,
        logoWidth,
        logoHeight,
        warpHeight,
        showWelcome,
        warpStyle
    };
};
