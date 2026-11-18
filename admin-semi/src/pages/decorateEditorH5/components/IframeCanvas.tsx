/**
 * 真实渲染画布（2026-11-18 Phase 2，方案见 ~/workspace/editor-unify/iframe-canvas-design.md）
 *
 * 画布中列改 iframe 嵌入前台 decoratePreview 页：渲染与线上首页/自定义页完全同一份
 * 代码（uniapp-react DecorateModules 分发器），编辑器只发数据（postMessage
 * decorate-data）、收布局（decorate-layout），点选/拖拽/操作条全部在父侧透明
 * 覆盖层完成，iframe 纯展示。
 *
 * 口径：
 * - 标题栏：前台分发器按 pageModule.title 自行渲染 navbar（与真前台一致），父侧
 *   不再画 PageHeaderPreview（避免双标题）；覆盖层在首模块上方留出标题区点击 → 页面设置。
 * - 隐藏模块（isShow=false）前台不渲染、无布局槽位：覆盖层按其顺序位置锚定一个
 *   模块名标签供点选/操作（显隐/删除/上下移），拖拽排序请用操作条或切兼容渲染。
 * - 页面背景：窗口底色沿用编辑器 pageBackgroundStyle（windowStyle 传入）；前台根
 *   节点按 pageModule.backgroundColor 自行着色（与真前台一致）。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Toast } from "@douyinfe/semi-ui";
import { IconEyeClosed } from "@douyinfe/semi-icons";
import { editorStore, useEditorState } from "../store/editorStore";
import type { DecorateModuleBase } from "../contract/types";
import ModuleOperateBar from "./ModuleOperateBar";
import {
    EMBED_ORIGIN,
    EMBED_URL,
    registerDropResolver,
    useLibraryDragState,
    type EmbedLayout,
    type EmbedLayoutItem
} from "./canvasBridge";
import { useI18n } from "../../../i18n";

const DRAG_THRESHOLD = 6;

export default function IframeCanvas({ preview = false, windowStyle }: { preview?: boolean; windowStyle?: React.CSSProperties }) {
    const { t } = useI18n();
    const s = useEditorState();
    const libDrag = useLibraryDragState();
    const iframeRef = useRef<HTMLIFrameElement>(null);
    const wrapRef = useRef<HTMLDivElement>(null);
    const [ready, setReady] = useState(false);
    const [layout, setLayout] = useState<EmbedLayout | null>(null);
    const layoutRef = useRef<EmbedLayout | null>(null);
    layoutRef.current = layout;
    const [reorder, setReorder] = useState<{ from: number; dy: number; insertPos: number } | null>(null);
    const lastDragEndRef = useRef(0);

    const modules = s.moduleList;
    const frameHeight = layout?.totalHeight ? Math.max(layout.totalHeight, 120) : 667;

    // ---------------- 与嵌入页通信 ----------------

    const sendData = useCallback(() => {
        const win = iframeRef.current?.contentWindow;
        if (!win) return;
        const st = editorStore.getState();
        try {
            win.postMessage({ type: "decorate-data", moduleList: st.moduleList, pageModule: st.pageModule }, EMBED_ORIGIN);
        } catch {
            /* iframe 未就绪时忽略，就绪握手会补发 */
        }
    }, []);
    const sendDataRef = useRef(sendData);
    sendDataRef.current = sendData;

    useEffect(() => {
        const onMessage = (e: MessageEvent) => {
            if (e.origin !== EMBED_ORIGIN) return;
            const d = e.data as any;
            if (!d || typeof d !== "object") return;
            if (d.type === "decorate-ready") {
                setReady(true);
                sendDataRef.current();
            } else if (d.type === "decorate-layout" && Array.isArray(d.items)) {
                const next: EmbedLayout = {
                    items: d.items
                        .filter((it: any) => typeof it?.index === "number")
                        .map((it: any) => ({ index: it.index, top: Number(it.top) || 0, height: Number(it.height) || 0 })),
                    totalHeight: Number(d.totalHeight) || 0
                };
                setLayout((prev) => (prev && JSON.stringify(prev) === JSON.stringify(next) ? prev : next));
            }
        };
        window.addEventListener("message", onMessage);
        return () => window.removeEventListener("message", onMessage);
    }, []);

    // 数据变更防抖 200ms 下发（不刷新 iframe）
    const sendTimer = useRef<number | undefined>(undefined);
    useEffect(() => {
        window.clearTimeout(sendTimer.current);
        sendTimer.current = window.setTimeout(sendData, 200);
        return () => window.clearTimeout(sendTimer.current);
    }, [s.moduleList, s.pageModule, sendData]);

    useEffect(() => {
        if (ready) sendData();
    }, [ready, sendData]);

    // ---------------- 模块库拖入：落点解析（真实模式） ----------------

    useEffect(() => {
        if (preview) return;
        registerDropResolver((clientX, clientY) => {
            const wrap = wrapRef.current;
            if (!wrap) return null;
            const r = wrap.getBoundingClientRect();
            if (clientX < r.left || clientX > r.right || clientY < r.top || clientY > r.bottom) return null;
            const relY = clientY - r.top;
            const items = layoutRef.current?.items ?? [];
            for (const it of items) {
                if (relY < it.top + it.height / 2) return { insertIndex: it.index, lineY: it.top };
            }
            const last = items[items.length - 1];
            return {
                insertIndex: editorStore.getState().moduleList.length,
                lineY: last ? last.top + last.height : layoutRef.current?.totalHeight ?? Math.round(r.height)
            };
        });
        return () => registerDropResolver(null);
    }, [preview]);

    // ---------------- 覆盖层槽位 ----------------

    const itemByIndex = useMemo(() => {
        const map = new Map<number, EmbedLayoutItem>();
        for (const it of layout?.items ?? []) {
            if (it.index < modules.length) map.set(it.index, it);
        }
        return map;
    }, [layout, modules.length]);

    /** 无槽位模块（隐藏/未注册）的锚点纵坐标：下一个有槽位模块的 top，否则上一个的底边 */
    const anchorTopFor = useCallback(
        (index: number): number => {
            for (let j = index + 1; j < modules.length; j++) {
                const it = itemByIndex.get(j);
                if (it) return it.top;
            }
            for (let j = index - 1; j >= 0; j--) {
                const it = itemByIndex.get(j);
                if (it) return it.top + it.height;
            }
            return layout?.totalHeight ?? 0;
        },
        [itemByIndex, modules.length, layout]
    );

    /** 指针纵坐标（覆盖层内）→ 可见槽位序列中的插入位（0..items.length） */
    const insertionPosForY = useCallback((relY: number): number => {
        const items = layoutRef.current?.items ?? [];
        for (let i = 0; i < items.length; i++) {
            if (relY < items[i].top + items[i].height / 2) return i;
        }
        return items.length;
    }, []);

    const commitReorder = useCallback(
        (from: number, insertPos: number) => {
            const items = layoutRef.current?.items ?? [];
            const fromPos = items.findIndex((it) => it.index === from);
            if (fromPos === -1) return;
            if (insertPos === fromPos || insertPos === fromPos + 1) return;
            let to: number;
            if (insertPos >= items.length) {
                to = editorStore.getState().moduleList.length - 1;
            } else {
                to = items[insertPos].index;
                if (from < to) to -= 1;
            }
            if (to === from) return;
            const result: any = editorStore.moveModule(from, to);
            if (result !== true) Toast.warning(t(String(result?.reason || "")));
        },
        [t]
    );

    const onSlotPointerDown = (e: React.PointerEvent, index: number) => {
        if (preview || e.button !== 0) return;
        const wrap = wrapRef.current;
        if (!wrap) return;
        const wrapTop = wrap.getBoundingClientRect().top;
        const startY = e.clientY;
        let dragging = false;

        const onMove = (ev: PointerEvent) => {
            const dy = ev.clientY - startY;
            if (!dragging && Math.abs(dy) > DRAG_THRESHOLD) dragging = true;
            if (dragging) {
                setReorder({ from: index, dy, insertPos: insertionPosForY(ev.clientY - wrapTop) });
            }
        };
        const onUp = (ev: PointerEvent) => {
            window.removeEventListener("pointermove", onMove, true);
            window.removeEventListener("pointerup", onUp, true);
            window.removeEventListener("pointercancel", onUp, true);
            if (dragging) {
                lastDragEndRef.current = Date.now();
                commitReorder(index, insertionPosForY(ev.clientY - wrapTop));
            }
            setReorder(null);
        };
        window.addEventListener("pointermove", onMove, true);
        window.addEventListener("pointerup", onUp, true);
        window.addEventListener("pointercancel", onUp, true);
    };

    const onSlotClick = (index: number) => {
        if (Date.now() - lastDragEndRef.current < 300) return; // 拖拽刚结束的 click 不改选中
        editorStore.selectModule(index);
    };

    const insertLineTop = (pos: number): number => {
        const items = layout?.items ?? [];
        if (pos < items.length) return items[pos].top;
        const last = items[items.length - 1];
        return last ? last.top + last.height : layout?.totalHeight ?? frameHeight;
    };

    const firstItemTop = layout?.items.length ? layout.items[0].top : 0;
    const isPageActive = s.editMode === "page" && !preview;

    const renderSlot = (m: DecorateModuleBase, index: number) => {
        const it = itemByIndex.get(index);
        const isActive = s.activeModuleIndex === index && s.editMode === "module" && !preview;
        const label = t(m.label || m.type);
        if (!it) {
            // 无槽位（隐藏模块前台不渲染）：锚定标签供点选与操作，不支持拖拽排序
            return (
                <div
                    key={m.moduleIndex}
                    className={`modules-item h5-editor-overlay-slotless${isActive ? " module-item-active" : ""}`}
                    style={{ top: anchorTopFor(index) }}
                    onClick={() => onSlotClick(index)}
                >
                    <div className="module-label module-label--handle" title={m.isShow === false ? t("已隐藏") : label}>
                        <div className="label-name">
                            {m.isShow === false ? (
                                <IconEyeClosed style={{ fontSize: 12, marginRight: 4, verticalAlign: -1 }} />
                            ) : null}
                            {label}
                        </div>
                    </div>
                    {isActive ? <ModuleOperateBar moduleData={m} index={index} /> : null}
                </div>
            );
        }
        const isDraggingThis = reorder?.from === index;
        return (
            <div
                key={m.moduleIndex}
                className={`modules-item h5-editor-overlay-slot${isActive ? " module-item-active" : ""}${isDraggingThis ? " is-dragging" : ""}`}
                style={{
                    top: it.top,
                    height: it.height,
                    transform: isDraggingThis ? `translateY(${reorder.dy}px)` : undefined,
                    zIndex: isDraggingThis ? 40 : undefined
                }}
                onPointerDown={(e) => onSlotPointerDown(e, index)}
                onClick={() => onSlotClick(index)}
            >
                <div className="module-ad-con" />
                <div className="module-label module-label--handle" title={t("拖拽排序")}>
                    <div className="label-name">
                        <span className="label-grip" aria-hidden="true">⋮⋮</span>
                        {label}
                    </div>
                </div>
                {isActive ? <ModuleOperateBar moduleData={m} index={index} /> : null}
            </div>
        );
    };

    return (
        <div className="decorate-page-window" style={windowStyle}>
            <div className="h5-editor-iframe-wrap" ref={wrapRef} style={{ height: frameHeight }}>
                <iframe
                    ref={iframeRef}
                    src={EMBED_URL}
                    title={t("H5 真实渲染")}
                    style={{ height: frameHeight }}
                    onLoad={() => sendData()}
                />
                {!ready && !preview ? <div className="h5-editor-iframe-loading">{t("正在加载真实前台…")}</div> : null}
                {!preview ? (
                    <div className="h5-editor-overlay">
                        {firstItemTop > 4 ? (
                            <div
                                className={`h5-editor-overlay-navbar${isPageActive ? " is-active" : ""}`}
                                style={{ height: firstItemTop }}
                                title={t("页面设置")}
                                onClick={() => editorStore.selectPage()}
                            />
                        ) : null}
                        {modules.map((m, index) => renderSlot(m, index))}
                        {modules.length === 0 ? (
                            <div className="h5-editor-canvas-empty">{t("请在左侧模块库点击添加模块")}</div>
                        ) : null}
                        {reorder ? <div className="h5-editor-insert-line" style={{ top: insertLineTop(reorder.insertPos) }} /> : null}
                        {libDrag.active && libDrag.lineY !== null ? (
                            <div className="h5-editor-insert-line" style={{ top: libDrag.lineY }} />
                        ) : null}
                    </div>
                ) : null}
            </div>
        </div>
    );
}
