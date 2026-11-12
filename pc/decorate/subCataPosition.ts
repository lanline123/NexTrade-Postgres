/**
 * 首页焦点区子类目弹层定位（Teleport + fixed）
 * 用视口坐标，避免 absolute + scrollY 在布局未稳 / 滚动容器差异下算出 (0,0)
 */

export type DomRectLike = Pick<DOMRect, "top" | "right" | "width" | "height">;

/**
 * 根据左侧分类面板的 getBoundingClientRect 生成 fixed 定位样式。
 * 宽高无效时返回 null，调用方应保留旧坐标或延后重试，避免弹层落到左上角。
 */
export function buildSubCataFixedStyle(rect: DomRectLike): Record<string, string> | null {
    if (!Number.isFinite(rect.top) || !Number.isFinite(rect.right) || rect.width <= 0 || rect.height <= 0) {
        return null;
    }

    return {
        position: "fixed",
        top: `${Math.round(rect.top)}px`,
        left: `${Math.round(rect.right) + 1}px`,
        zIndex: "9999"
    };
}
