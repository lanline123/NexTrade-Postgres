/**
 * 会员中心顶部背景自定义（我方新增；论坛帖 257746 子项，原版仅列入
 * 需求建议表未上线）——pageModule.topBackground 的解析口径，编辑器
 * （decorateEditorH5 页面设置写入、Canvas 兼容画布镜像解析）与本页共用
 * 同一字段契约：
 * - type 1 纯色：{ color }
 * - type 2 渐变：{ gradient: { colorA, colorB, gradientType } }（镜像会员
 *   模块既有 gradient 口径，缺省色/方向与 userInfo 默认渐变一致）
 * - type 3 背景图：{ image }（PicSelect 单图对象 { picUrl } 或纯字符串）
 * 未配置、type 缺省/0 或值无效时返回 null，页面保持现行默认样式零变化。
 */

export interface MemberTopBackground {
    kind: "color" | "gradient" | "image";
    /** kind=color/gradient 时的基础层 CSS background-image 值（不含渐隐层） */
    background?: string;
    /** kind=image 时的图片原始地址（调用方再过 imageFormat 解析存储域名） */
    image?: string;
    /** 渐隐目标色：页面背景色 backgroundColor，缺省 #f5f5f5（与沉浸式渐隐目标一致） */
    fadeColor: string;
}

const GRADIENT_DIR_MAP: Record<string, string> = { upDown: "to bottom", leftRight: "to right", diagonal: "135deg" };

export function resolveMemberTopBackground(pageModule: Record<string, any> | null | undefined): MemberTopBackground | null {
    const tb = pageModule?.topBackground;
    if (!tb || typeof tb !== "object") return null;
    const fadeColor = (typeof pageModule?.backgroundColor === "string" && pageModule.backgroundColor) || "#f5f5f5";
    const type = Number(tb.type ?? 0);
    if (type === 1) {
        const color = typeof tb.color === "string" ? tb.color.trim() : "";
        if (!color) return null;
        return { kind: "color", background: `linear-gradient(${color}, ${color})`, fadeColor };
    }
    if (type === 2) {
        const g: Record<string, any> = tb.gradient && typeof tb.gradient === "object" ? tb.gradient : {};
        const colorA = g.colorA || "#fcf6fc";
        const colorB = g.colorB || "#f3f3fe";
        const dir = GRADIENT_DIR_MAP[String(g.gradientType || "leftRight")] || "to right";
        return { kind: "gradient", background: `linear-gradient(${dir}, ${colorA}, ${colorB})`, fadeColor };
    }
    if (type === 3) {
        const raw = tb.image;
        const url = typeof raw === "string" ? raw : raw && typeof raw === "object" ? raw.picUrl || "" : "";
        if (!url) return null;
        return { kind: "image", image: url, fadeColor };
    }
    return null;
}

/** 纯色/渐变两类的完整 background-image：上层渐隐（透明→页面背景）+ 基础层 */
export function memberTopBackgroundCss(tb: MemberTopBackground): string {
    return `linear-gradient(to bottom, transparent 55%, ${tb.fadeColor} 100%), ${tb.background}`;
}

/** 背景图类的渐隐覆盖层 background（盖在图片上，底部渐隐至页面背景） */
export function memberTopBackgroundFadeCss(tb: MemberTopBackground): string {
    return `linear-gradient(to bottom, transparent 55%, ${tb.fadeColor} 100%)`;
}
