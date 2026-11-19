/**
 * 真实渲染画布（2026-11-18 Phase 2，方案见 ~/workspace/editor-unify/iframe-canvas-design.md）
 *
 * 画布中列改 iframe 嵌入前台 decoratePreview 页：渲染与线上首页/自定义页完全同一份
 * 代码（uniapp-react DecorateModules 分发器），编辑器只发数据（postMessage
 * decorate-data）、收布局（decorate-layout），点选/拖拽/操作条全部在父侧透明
 * 覆盖层完成，iframe 纯展示。
 *
 * 口径：
 * - 标题栏：父侧在 iframe 之上渲染与兼容模式同一个 PageHeaderPreview（Canvas 的
 *   真实模式分支，bare 模式挂载）——嵌入页不渲染编辑器口径的标题栏（2026-11-18
 *   线上验收实证：真实模式曾直接从首模块开始、无状态栏图与居中标题，「前台自带
 *   navbar、父侧不重复画」的旧假设不成立，已废止）。覆盖层坐标全部相对 iframe
 *   内容（本组件的 wrap 内部），标题栏在 wrap 之外、之上，不影响槽位对齐；覆盖层
 *   顶部点击区（首模块 top 之上的区域）仍映射到页面设置，与标题栏点击口径一致。
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
import {
    markEmbedDataSent,
    markEmbedIframeLoaded,
    markEmbedLayout,
    markEmbedReady,
    resetEmbedReadiness
} from "./embedReadiness";
import { useI18n } from "../../../i18n";

const DRAG_THRESHOLD = 6;

export default function IframeCanvas({
    preview = false,
    windowStyle,
    bare = false,
    active = true
}: {
    preview?: boolean;
    windowStyle?: React.CSSProperties;
    /** bare=true 时不渲染 .decorate-page-window 外框（由 Canvas 真实模式分支统一渲染外框 + PageHeaderPreview） */
    bare?: boolean;
    /**
     * active=false 时本画布被保活隐藏（兼容渲染模式下，2026-11-19 起两模式
     * 共存挂载、切回真实模式不再重载 iframe）：不注册模块库落点解析（归当前
     * 可见画布），其余通信与布局回传照常，方便切回即已就绪。
     */
    active?: boolean;
}) {
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
        // seq 随每次下发递增：供就绪状态机判定 layout 对应哪一轮数据；嵌入页
        // 将来回带 seq 时可精确对齐（见 embedReadiness 的 ready 依赖点说明）。
        const seq = markEmbedDataSent();
        try {
            // shopId 随数据下发（店铺模板页才有值）：嵌入页据此拉真实店铺详情喂
            // shopInfo 等店铺模块；旧版嵌入页忽略未知字段，向后兼容。
            // pageType 随数据下发（2026-11-19 整页配置型页型修复）：登录(5)/搜索(6)/
            // 商品详情(8)的模块是「整页配置型」——前台页面本体消费 module 作整页
            // 配置、没有积木渲染器，嵌入页须按 pageType 走整页分支渲染真实页面；
            // 此前不下发 pageType 时嵌入页无从分支，三类页真实渲染整页空白。
            // 旧版嵌入页忽略未知字段，向后兼容。
            win.postMessage(
                { type: "decorate-data", seq, moduleList: st.moduleList, pageModule: st.pageModule, shopId: st.shopId || 0, pageType: st.pageType || 0 },
                EMBED_ORIGIN
            );
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
                markEmbedReady();
                setReady(true);
                sendDataRef.current();
            } else if (d.type === "decorate-layout" && Array.isArray(d.items)) {
                // layout 是嵌入页渲染完成后的测量回传：加载门据此判定首帧已渲染
                // （ready 信号的唯一依赖点，见 embedReadiness.ts）。
                markEmbedLayout(typeof d.seq === "number" ? d.seq : undefined);
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
        if (preview || !active) return;
        return registerDropResolver((clientX, clientY) => {
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
    }, [preview, active]);

    // 画布真正卸载（详情加载失败切错误卡、离开编辑器）时复位就绪状态机：
    // 下次挂载的是一张全新 iframe，旧会话的 ready/layout 不能沿用。
    useEffect(() => {
        return () => resetEmbedReadiness();
    }, []);

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

    const wrapContent = (
            <div className="h5-editor-iframe-wrap" ref={wrapRef} style={{ height: frameHeight }}>
                <iframe
                    ref={iframeRef}
                    src={EMBED_URL}
                    title={t("H5 真实渲染")}
                    style={{ height: frameHeight }}
                    onLoad={() => {
                        markEmbedIframeLoaded();
                        sendData();
                    }}
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
    );
    if (bare) return wrapContent;
    return (
        <div className="decorate-page-window" style={windowStyle}>
            {wrapContent}
        </div>
    );
}
