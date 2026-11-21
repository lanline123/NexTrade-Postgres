/**
 * 新建页模板库 · 内置模板定义与组装（原版有·我方补齐，数据层口径见下）
 *
 * 原版口径（v5.8.61 实证）：
 * - 模板库本体是 Tigshop 官网模板市场：admin `decorate/mobileDecorate/
 *   TemplateLibrary.vue` 经本站后端 `decorate/template/category|list|detail`
 *   转发官网 OpenTemplate Partner API（缺省 https://api.tigshop.com/index），
 *   靠站点授权码取数，模板数据（pageSn + 整页 JSON）存于官网云端、本站不可维护；
 * - 构成：按「页面」维度陈列，pageType 枚举映射 1→home / 2→member / 3→home
 *   （活动页复用首页模板）/ 4→category / 5→login / 6→search / 7→nav；
 *   筛选维度为行业（分类维度 6）/ 节日（19）/ 颜色（28 色点）三行 + 模板编码搜索；
 *   网格第一张卡恒为「新建空白模板」。
 *
 * 我方适配（形态同口径、数据自有）：
 * - 我方与官网无授权关系，不转发其商业模板数据；模板改为**内置定义**：
 *   每套模板只声明模块类型序列，套用时经注册表 createModuleInstance 以各模块
 *   演示默认值（module-defaults-fill 已充实、自有 OSS 已验资产）现组装整页
 *   { moduleList, pageModule }，与手工逐个添加模块所得完全同构；
 * - 分类维度沿用原版三行（行业 / 节日 / 颜色），选项集为我方内置小集合；
 * - pageType 映射沿用原版（3 活动页复用 home 模板）；原版映射表无 8（商品
 *   详情，落 home 兜底），我方改为映射到三风格模板（A1/A2/A3 本就是详情页的
 *   风格起点，比落首页模板更贴本页型语义）；pageType 7（底部导航）不在本
 *   编辑器页型集合内，无对应模板；
 * - 后台可维护层：原版亦非本站可维护（官网云端维护），我方本批为内置先行，
 *   模板增删改随版本发布；如需运营自助维护模板，另行立项（本地模板表 +
 *   管理页），不在本批。
 */

import { createModuleInstance } from "../../contract/moduleRegistry";
import type { DecorateModuleBase, PageModule } from "../../contract/types";

/** 模板适用范围（沿用原版官网 pageType 枚举命名，8 商品详情为我方映射，见文件头） */
export type TemplateScope = "home" | "member" | "category" | "login" | "search" | "detail";

/** 编辑器 pageType → 模板范围（原版 PAGE_TYPE_MAP 同口径 + 8 的我方映射） */
const PAGE_TYPE_SCOPE: Record<number, TemplateScope> = {
    1: "home",
    2: "member",
    3: "home",
    4: "category",
    5: "login",
    6: "search",
    8: "detail"
};

export function templateScopeForPageType(pageType: number): TemplateScope {
    return PAGE_TYPE_SCOPE[pageType] ?? "home";
}

export interface BuiltinTemplate {
    /** 模板编码（搜索口径同原版：按编码/名称匹配） */
    sn: string;
    name: string;
    scopes: TemplateScope[];
    industry: string;
    festival: string;
    /** 主题色（颜色筛选维度，色点展示） */
    color: string;
    colorName: string;
    /** 模块类型序列：套用时按序以注册 defaults 组装 */
    modules: string[];
    pageModule?: PageModule;
}

export const BUILTIN_TEMPLATES: BuiltinTemplate[] = [
    // ==================== 首页（活动页共用） ====================
    {
        sn: "NT-HOME-01",
        name: "通用商城首页",
        scopes: ["home"],
        industry: "综合百货",
        festival: "日常",
        color: "#65c4aa",
        colorName: "主题绿",
        modules: ["banner", "imageNav", "notice", "coupon", "productA1", "imageSquareAd", "productSlide"]
    },
    {
        sn: "NT-HOME-02",
        name: "生鲜果蔬首页",
        scopes: ["home"],
        industry: "生鲜食品",
        festival: "日常",
        color: "#3aa94c",
        colorName: "绿色",
        modules: ["banner", "catNav", "notice", "limitedSpecial", "productSlide", "imageAd", "product"]
    },
    {
        sn: "NT-HOME-03",
        name: "数码家电首页",
        scopes: ["home"],
        industry: "数码家电",
        festival: "日常",
        color: "#2b6cb0",
        colorName: "蓝色",
        modules: ["banner", "imageNav", "imageAd", "groupSliding", "product", "imageShowcase"]
    },
    {
        sn: "NT-HOME-04",
        name: "节日大促首页",
        scopes: ["home"],
        industry: "综合百货",
        festival: "双十一",
        color: "#d92c2c",
        colorName: "红色",
        modules: ["banner", "countdown", "coupon", "seckill", "whiteLine", "productGrouping", "groupon"],
        pageModule: { backgroundColor: "#faf3ee" }
    },
    {
        sn: "NT-HOME-05",
        name: "新人专享首页",
        scopes: ["home"],
        industry: "综合百货",
        festival: "日常",
        color: "#e8912d",
        colorName: "橙色",
        modules: ["banner", "newcomerCombo", "imageNav", "productA1", "imageAd", "productSlide"]
    },
    // ==================== 会员中心 ====================
    {
        sn: "NT-MEMBER-01",
        name: "会员中心 · 新版",
        scopes: ["member"],
        industry: "综合百货",
        festival: "日常",
        color: "#65c4aa",
        colorName: "主题绿",
        modules: ["userInfoV2", "myWalletV2", "myService", "adImage"]
    },
    {
        sn: "NT-MEMBER-02",
        name: "会员中心 · 经典",
        scopes: ["member"],
        industry: "综合百货",
        festival: "日常",
        color: "#2b6cb0",
        colorName: "蓝色",
        modules: ["userInfo", "myWallet", "orderEntry", "myService"]
    },
    // ==================== 分类中心 ====================
    {
        sn: "NT-CATEGORY-01",
        name: "分类中心 · 样式一",
        scopes: ["category"],
        industry: "综合百货",
        festival: "日常",
        color: "#65c4aa",
        colorName: "主题绿",
        modules: ["cateCenterV1"]
    },
    {
        sn: "NT-CATEGORY-02",
        name: "分类中心 · 样式二",
        scopes: ["category"],
        industry: "综合百货",
        festival: "日常",
        color: "#2b6cb0",
        colorName: "蓝色",
        modules: ["cateCenterV2"]
    },
    // ==================== 登录页 ====================
    {
        sn: "NT-LOGIN-01",
        name: "登录页 · 标准",
        scopes: ["login"],
        industry: "综合百货",
        festival: "日常",
        color: "#65c4aa",
        colorName: "主题绿",
        modules: ["mobileLogin"]
    },
    // ==================== 搜索结果页 ====================
    {
        sn: "NT-SEARCH-01",
        name: "搜索结果页 · 标准",
        scopes: ["search"],
        industry: "综合百货",
        festival: "日常",
        color: "#65c4aa",
        colorName: "主题绿",
        modules: ["mobileSearchResult"]
    },
    // ==================== 商品详情（三风格起点） ====================
    {
        sn: "NT-DETAIL-01",
        name: "商品详情 · 标准风",
        scopes: ["detail"],
        industry: "综合百货",
        festival: "日常",
        color: "#65c4aa",
        colorName: "主题绿",
        modules: ["productDetailA1"]
    },
    {
        sn: "NT-DETAIL-02",
        name: "商品详情 · 门店风",
        scopes: ["detail"],
        industry: "综合百货",
        festival: "日常",
        color: "#3aa94c",
        colorName: "绿色",
        modules: ["productDetailA2"]
    },
    {
        sn: "NT-DETAIL-03",
        name: "商品详情 · 跨境风",
        scopes: ["detail"],
        industry: "综合百货",
        festival: "日常",
        color: "#2b6cb0",
        colorName: "蓝色",
        modules: ["productDetailA3"]
    }
];

/** 按范围取模板（保持定义顺序） */
export function templatesForScope(scope: TemplateScope): BuiltinTemplate[] {
    return BUILTIN_TEMPLATES.filter((tpl) => tpl.scopes.includes(scope));
}

/**
 * 组装模板整页数据：逐个 createModuleInstance（注册 defaults 现取，含演示
 * 内容与自有 OSS 资产），未注册类型跳过；moduleIndex 在此预置（applyTemplate
 * 链不再分配），形态与手工添加一致。
 */
export function buildTemplateData(tpl: BuiltinTemplate): { moduleList: DecorateModuleBase[]; pageModule: PageModule } {
    const base = Date.now();
    const moduleList: DecorateModuleBase[] = [];
    tpl.modules.forEach((type, i) => {
        const instance = createModuleInstance(type);
        if (instance) {
            moduleList.push({ ...instance, moduleIndex: base + i } as DecorateModuleBase);
        }
    });
    return { moduleList, pageModule: tpl.pageModule ? { ...tpl.pageModule } : {} };
}

/** 从模块数据里找第一张图（picList / bannerContent.picList / picUrl 对象或字符串） */
function firstPicInModule(moduleData: Record<string, any> | undefined): string {
    if (!moduleData) return "";
    const fromList = (list: any): string => {
        if (!Array.isArray(list) || !list.length) return "";
        const item = list[0];
        return item?.picUrl || item?.picThumb || "";
    };
    return (
        fromList(moduleData.picList) ||
        fromList(moduleData.bannerContent?.picList) ||
        (typeof moduleData.picUrl === "string" ? moduleData.picUrl : moduleData.picUrl?.picUrl || moduleData.picUrl?.picThumb || "") ||
        ""
    );
}

/**
 * 模板封面：取组装后第一个带图模块的首图——封面即模板自身内容的真实首图，
 * 资产均为各模块演示默认值（自有 OSS 已验），无图模板返回空串走占位块。
 */
export function templateCover(tpl: BuiltinTemplate): string {
    const { moduleList } = buildTemplateData(tpl);
    for (const item of moduleList) {
        const pic = firstPicInModule(item.module as Record<string, any>);
        if (pic) return pic;
    }
    return "";
}
