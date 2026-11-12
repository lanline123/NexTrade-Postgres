import { onMounted, onUnmounted, ref } from "vue";

interface UseResponsiveGridColumnsOptions {
    /** 单卡片密度基准宽度（px），用于按容器宽度推导列数 */
    minCardWidth?: number;
    /** 网格列间距（px），须与 CSS grid 的 column-gap 一致，否则列数会算偏 */
    gap?: number;
    /** 列数下限 */
    minColumns?: number;
    /** 列数上限 */
    maxColumns?: number;
}

/**
 * 按容器实际可用宽度推导网格列数，配合 `grid-template-columns: repeat(var(--x), minmax(0, 1fr))` 使用。
 * 用「最小卡片宽 + 列间距」为基准算列数，列宽交给 grid 弹性均分铺满，
 * 从根源规避「固定卡片宽 + flex-wrap」造成的列数僵硬跳变与行尾留白。
 *
 * @returns containerRef 绑定到网格容器；columns 当前列数（绑到 CSS 变量驱动 grid）
 */
export function useResponsiveGridColumns(options: UseResponsiveGridColumnsOptions = {}) {
    const { minCardWidth = 240, gap = 16, minColumns = 2, maxColumns = 6 } = options;

    const containerRef = ref<HTMLElement | null>(null);
    const columns = ref(minColumns);
    let resizeObserver: ResizeObserver | null = null;

    const update = () => {
        const width = containerRef.value?.clientWidth ?? 0;
        if (width <= 0) return;
        const next = Math.floor((width + gap) / (minCardWidth + gap));
        columns.value = Math.min(maxColumns, Math.max(minColumns, next));
    };

    onMounted(() => {
        update();
        if (typeof ResizeObserver !== "undefined" && containerRef.value) {
            resizeObserver = new ResizeObserver(() => update());
            resizeObserver.observe(containerRef.value);
        }
    });

    onUnmounted(() => {
        resizeObserver?.disconnect();
    });

    return { containerRef, columns };
}
