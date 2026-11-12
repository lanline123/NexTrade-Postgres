/**
 * 装修模块类型归一化（前台 · PC 首页新套）
 *
 * 页面级分流下渲染器只处理「全 kebab」的新版页面，故仅做：kebab 透传 + kebab 历史别名。
 * 不引入 vortmall 的 camelCase→kebab 映射——旧 camelCase 首页走旧 v-if 链、不进本渲染器，
 * 若照搬会把旧 `pcImageAd` 误拉进新 `pc-image-ad`。
 */

export interface DecorateModuleLike {
    type?: string;
    url?: string;
    isShow?: boolean;
    module?: unknown;
    moduleIndex?: number | string;
}

// kebab 历史别名 → Admin 现行 kebab 标准 type
const KEBAB_ALIAS_TO_KEBAB_PC: Record<string, string> = {
    "pc-category-nav-3": "pc-category-nav3",
    "pc-white-line": "pc-divider",
    "pc-white-blank": "pc-spacer"
};

export function normalizeModuleType(module: DecorateModuleLike): string {
    const rawType = String(module?.type ?? "");
    if (!rawType) return "";
    if (KEBAB_ALIAS_TO_KEBAB_PC[rawType]) return KEBAB_ALIAS_TO_KEBAB_PC[rawType];
    return rawType;
}
