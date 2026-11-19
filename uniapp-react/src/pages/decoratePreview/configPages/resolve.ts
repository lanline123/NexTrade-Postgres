/**
 * 整页配置型页型的分支判定（2026-11-19 登录/搜索/商品详情真实渲染空白修复）。
 *
 * 登录页（pageType=5）/ 搜索页（6）/ 商品详情页（8）的装修模块与首页积木模块
 * 本质不同：它们是「整页配置型」——真实前台页面（pages/login、pages/search/result、
 * pages/product）把 module 当整页配置消费（登录配色边距、搜索结果样式、详情
 * A1/A2/A3 版式与底栏），没有可注册进 MODULE_REGISTRY 的积木渲染器。此前
 * decoratePreview 只走通用分发器 DecorateModules，这三类 type 一律未注册被
 * 跳过（0 槽位），且编辑器下发的 decorate-data 不带 pageType、预览页无从分支，
 * 导致三类页在编辑器真实渲染下整页空白（10103/10104/10105 浏览器实证）。
 *
 * 本文件只做纯判定：pageType ∈ {5,6,8} 且 moduleList 含对应配置型模块（可见）
 * 时命中整页分支，预览页改渲染真实页面组件（配置用编辑器实时下发数据，不走
 * 接口取已发布版，口径与 shopInfo 的 shopId 上下文下发先例一致）。不命中
 * （其它页型、配置模块缺失/隐藏）一律返回 null，预览页回退现有通用分发，
 * pt1/2/3 等既有路径行为不变。
 */

/** 页型 → 该页型的整页配置模块 type 集合（一页一配置，编辑器侧同口径限定） */
export const CONFIG_PAGE_MODULE_TYPES: Record<number, readonly string[]> = {
    5: ["mobileLogin"],
    6: ["mobileSearchResult"],
    8: ["productDetailA1", "productDetailA2", "productDetailA3"]
};

export interface ConfigPageBranch {
    /** 命中的页型（5 登录 / 6 搜索 / 8 商品详情） */
    pageType: number;
    /** 配置模块在 moduleList 中的下标（整页分支占用的唯一槽位） */
    index: number;
    /** 配置模块的 type（pt8 据此区分 A1/A2/A3 版式） */
    type: string;
    /** 配置模块的 module 数据（编辑器实时数据，原样透传给页面组件作配置） */
    module: any;
}

/**
 * 分支判定：找到 moduleList 中第一个可见的本页型配置模块即命中。
 * 可见口径与 decoratePreview 既有槽位计算（visibleSlotIndices）一致：isShow 为真。
 */
export function resolveConfigPageBranch(pageType: number, modules: any[] | undefined | null): ConfigPageBranch | null {
    const types = CONFIG_PAGE_MODULE_TYPES[pageType];
    if (!types || !Array.isArray(modules)) return null;
    for (let i = 0; i < modules.length; i++) {
        const mod = modules[i];
        if (!mod?.isShow) continue;
        if (types.includes(mod.type)) {
            return { pageType, index: i, type: mod.type, module: mod.module ?? {} };
        }
    }
    return null;
}

/**
 * 整页分支命中时的槽位下标集合：整页内容只占 1 个槽位，对应配置模块下标，
 * 使编辑器覆盖层点选能选中该模块、右侧设置照常绑定。不命中返回 null，
 * 调用方回退常规槽位计算。
 */
export function configPageSlotIndices(pageType: number, modules: any[] | undefined | null): number[] | null {
    const branch = resolveConfigPageBranch(pageType, modules);
    return branch ? [branch.index] : null;
}
