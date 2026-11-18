/**
 * 轮播定高（编辑器预览共用，2026-11-18 首焦轮播同类问题批量修复引入）。
 *
 * NutUI Swiper 不定高时按每页内容的自然高度反复重测：图片未加载完高度为 0、
 * 加载后骤增，自动播放切换不同比例图片时模块高度随之跳变、下方内容被推来推去、
 * 指示点被挤出视野（banner 首发，bannerA2 / imageAd / imageAd2 同构同病）。
 * 口径与前台一致：按首图宽高比锁定高度，图片 cover 填满定高区。
 *
 * 用法：把 wrapRef 挂到轮播外层容器，height 有值时写进容器 style；
 * 配套 CSS 需让 .nut-swiper / .nut-swiper-item / 幻灯内容 height:100%。
 * columns/gap 用于多图同页（分组轮播）：按单元格宽度算高。
 */
import { useEffect, useRef, useState } from "react";

export function useSwiperHeight(
    firstPicUrl: string | undefined,
    opts?: { columns?: number; gap?: number; fallbackRatio?: number; min?: number; max?: number }
) {
    const wrapRef = useRef<HTMLDivElement | null>(null);
    const [width, setWidth] = useState(0);
    const [ratio, setRatio] = useState(opts?.fallbackRatio ?? 2);

    // 编辑器画布宽度固定，挂载后测一次容器实际宽度（含各级 padding 的真实可用宽）
    useEffect(() => {
        if (wrapRef.current) {
            const w = wrapRef.current.clientWidth;
            if (w > 0) setWidth((prev) => (prev === w ? prev : w));
        }
    });

    // 首图地址变化时重测宽高比（用离屏 Image，不依赖幻灯内 img 的加载时序）
    useEffect(() => {
        if (!firstPicUrl) return;
        let alive = true;
        const im = new Image();
        im.onload = () => {
            if (alive && im.naturalWidth > 0 && im.naturalHeight > 0) {
                setRatio(im.naturalWidth / im.naturalHeight);
            }
        };
        im.src = firstPicUrl;
        return () => {
            alive = false;
        };
    }, [firstPicUrl]);

    const columns = Math.max(1, opts?.columns ?? 1);
    const gap = opts?.gap ?? 0;
    const cellWidth = width > 0 ? (width - (columns - 1) * gap) / columns : 0;
    const height =
        cellWidth > 0 && ratio > 0
            ? Math.max(opts?.min ?? 40, Math.min(opts?.max ?? 375, Math.round(cellWidth / ratio)))
            : undefined;
    return { wrapRef, height };
}
