/**
 * 确定性轮播（编辑器预览专用，2026-11-18）。
 *
 * NutUI Swiper 以 react-spring 弹簧动画驱动幻灯位置：动画帧不推进时（后台标签页、
 * 无头/弱设备浏览器）会出现首图闪烁、自动播放切不动、第 2/3 页始终空白
 * （2026-11-18 admin v54 线上验收实测）。编辑器预览只需忠实呈现当前页与
 * 自动轮播节奏，故自研极简实现：幻灯绝对堆叠、当前页 opacity 1、
 * setInterval 推进，不依赖动画帧。高度由外层容器决定（见 useSwiperHeight）。
 */
import { useEffect, useRef, useState, type ReactNode } from "react";

export default function SimpleCarousel({
    children,
    autoplayMs = 0,
    onChange,
}: {
    children: ReactNode[];
    autoplayMs?: number;
    onChange?: (index: number) => void;
}) {
    const count = children.length;
    const [index, setIndex] = useState(0);
    const indexRef = useRef(0);
    const onChangeRef = useRef(onChange);
    onChangeRef.current = onChange;

    useEffect(() => {
        indexRef.current = 0;
        setIndex(0);
    }, [count]);

    useEffect(() => {
        if (!autoplayMs || count <= 1) return;
        const timer = window.setInterval(() => {
            const next = (indexRef.current + 1) % count;
            indexRef.current = next;
            setIndex(next);
            onChangeRef.current?.(next);
        }, autoplayMs);
        return () => window.clearInterval(timer);
    }, [autoplayMs, count]);

    return (
        <div style={{ position: "relative", width: "100%", height: "100%", overflow: "hidden" }}>
            {children.map((child, i) => (
                <div
                    key={i}
                    style={{
                        position: "absolute",
                        inset: 0,
                        opacity: i === index ? 1 : 0,
                        transition: "opacity 0.45s ease",
                        pointerEvents: "none",
                    }}
                >
                    {child}
                </div>
            ))}
        </div>
    );
}
