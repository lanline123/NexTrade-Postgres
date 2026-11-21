/**
 * 手机画布（移植自 view/admin editor/components/CanvasViewport.vue + ModuleRenderer.vue）
 *
 * 壳层对齐原版（editor/Index.vue + src/css/decorate.less）：
 * - 画布窗 .decorate-page-window：390px（2026-11-21 由 375 改版，现代主流机型尺寸）、margin 50px auto、阴影 0 0 14px rgba(0,0,0,.1)、无圆角
 * - 选中/悬停描边由 .module-ad-con:before 承担（2px solid #155bd4 / 1px dashed，见 decorate.css）
 * - 模块名标签 .module-label：画布左侧外 −100px 白底 chip（带右箭头）
 * - 操作条 .module-operate：右侧外 −40px 38px 白底竖排，上移/下移/显隐/复制/删除（fixed 隐藏后三项），hover 出 .opt-tip
 * - 隐藏模块：.module-mask rgba(255,255,255,.6) 遮罩 + 居中闭眼图标
 * - 画布首项：页面头模拟件（原版 MobilePageHeaderPreview），沉浸式/头部风格与页面设置联动
 * - 悬浮「页面设置」按钮 .decorate-page-config-warp（fixed right 400px / top 110px）
 * - dnd-kit 纵向拖拽排序（约束校验失败弹提示并回弹）
 * 预览态（preview=true）隐藏全部编辑外壳，只留画布内容。
 *
 * 2026-11-18 Phase 2：画布分「真实渲染 / 兼容渲染」两模式（默认真实渲染）——
 * 真实渲染 = IframeCanvas（iframe 嵌入前台 decoratePreview，见 IframeCanvas.tsx）；
 * 兼容渲染 = 本文件旧画布（兜底保留）。模式存 localStorage（h5-editor-render-mode）。
 * 2026-11-18 v59 补漏：真实渲染同样在 iframe 之上渲染 PageHeaderPreview（与兼容
 * 模式同一组件、同一显隐规则）——嵌入页并不渲染编辑器口径的页面标题栏（线上验收
 * 实证：真实模式画布直接从首模块开始、无状态栏图与居中标题，旧假设不成立）。
 * 2026-11-19 保活：真实分支挂载后不再随模式切换卸载（切兼容仅隐藏），切回
 * 真实模式不再重载 iframe；入口加载门见 decorateEditorH5/index.tsx 与
 * components/embedReadiness.ts。
 * 两种模式都注册模块库拖入的落点解析（真实模式在 IframeCanvas 内注册）：
 * 兼容模式按旧画布 DOM 位置算插入下标，见下方 useEffect。
 */
import { Suspense, useEffect, useMemo, useRef, useState } from "react";
import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Toast } from "@douyinfe/semi-ui";
import { IconEyeClosed, IconSetting } from "@douyinfe/semi-icons";
import { editorStore, useEditorState } from "../store/editorStore";
import { getLazyModuleComponent } from "../contract/lazyModule";
import ModuleErrorBoundary from "./ModuleErrorBoundary";
import ModuleOperateBar from "./ModuleOperateBar";
import IframeCanvas from "./IframeCanvas";
import { RENDER_MODE_STORAGE_KEY, getStoredRenderMode, registerDropResolver, useLibraryDragState } from "./canvasBridge";
import type { DecorateModuleBase } from "../contract/types";

import { useI18n } from '../../../i18n';
/** 预览组件懒加载（缓存/失败逐出/部署切换自愈在 contract/lazyModule 统一处理） */
function getLazyPreview(type: string, url?: string): React.ComponentType<any> | null {
    return getLazyModuleComponent("preview", type, url);
}

/** 页面背景（与 view/admin editor/Index.vue format() 口径一致） */
function pageBackgroundStyle(pageModule: Record<string, any>, pageType: number): React.CSSProperties {
    const backgroundRepeat: Record<number, string> = { 1: "no-repeat", 2: "repeat", 3: "repeat-y", 4: "repeat-x", 5: "no-repeat" };
    const backgroundSize: Record<number, string> = { 1: "100% auto", 2: "auto 100%", 3: "100% 100%", 4: "100% auto" };
    let pageBg = pageModule?.backgroundColor as string | undefined;
    if (pageType === 6) {
        // 搜索页无页面设置入口：取搜索结果模块的 backgroundColor（与旧编辑器一致）
        const searchMod = editorStore.getState().moduleList.find((m) => m.type === "mobileSearchResult");
        pageBg = (searchMod?.module as { backgroundColor?: string } | undefined)?.backgroundColor || pageBg || "#f5f5f5";
    }
    const style: Record<string, string> = {
        backgroundColor: pageBg ? "background-color:" + pageBg + ";" : "",
        backgroundImage: pageModule?.backgroundImage?.picUrl ? "background-image: url(" + pageModule.backgroundImage.picUrl + ");" : "",
        backgroundRepeat: "background-repeat:" + (backgroundRepeat[pageModule?.backgroundRepeat as number] ?? "") + ";",
        backgroundSize: "background-size:" + (backgroundSize[pageModule?.backgroundSize as number] ?? "") + ";"
    };
    const out: Record<string, string> = {};
    for (const decl of Object.values(style).join("").split(";")) {
        const idx = decl.indexOf(":");
        if (idx === -1) continue;
        const key = decl.slice(0, idx).trim();
        const value = decl.slice(idx + 1).trim();
        if (!key || !value) continue;
        out[key.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase())] = value;
    }
    return out as React.CSSProperties;
}

/** 页面头模拟件（原版 src/modules/src/commonPage/View.vue，经 MobilePageHeaderPreview 包装） */
function PageHeaderPreview({ preview }: { preview?: boolean }) {
    const s = useEditorState();
    // 原版 Index.vue：pageType 6（搜索）/8（商品详情）不渲染页面头
    if (s.pageType === 6 || s.pageType === 8) return null;
    const immersive = s.pageType === 2 && !!s.pageModule.immersive;
    const transparent = !immersive && Number((s.pageModule as Record<string, any>).headerStyle) === 2;
    const isActive = s.editMode === "page" && !preview;
    const classes = [
        "modules-item",
        "modules-item-topbar",
        isActive ? "module-item-active" : "",
        immersive ? "modules-item-topbar--immersive" : "",
        transparent ? "modules-item-topbar--transparent" : ""
    ]
        .filter(Boolean)
        .join(" ");
    return (
        <div
            className={classes}
            onClick={() => {
                if (!preview) editorStore.selectPage();
            }}
        >
            <div
                className="module-ad-con module-topbar-warp"
                style={s.pageModule.titleBackgroundColor ? { backgroundColor: s.pageModule.titleBackgroundColor } : undefined}
            >
                <div className="module-topbar-con">
                    <div className="module-topbar-title-con">
                        <div className="title-item" style={s.pageModule.titleColor ? { color: s.pageModule.titleColor } : undefined}>
                            {s.pageModule.title || "页面标题"}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}

function ModuleItem({ moduleData, index, preview }: { moduleData: DecorateModuleBase; index: number; preview?: boolean }) {
    const { t } = useI18n();
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: moduleData.moduleIndex, disabled: !!preview });
    const Preview = getLazyPreview(moduleData.type, moduleData.url);
    const isActive = editorStore.getState().activeModuleIndex === index && editorStore.getState().editMode === "module";
    const isHidden = moduleData.isShow === false;

    const style: React.CSSProperties = {
        transform: CSS.Translate.toString(transform),
        transition,
        zIndex: isDragging ? 10 : undefined
    };

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={`list-item modules-item${isActive && !preview ? " module-item-active" : ""}`}
            onClick={() => {
                if (!preview) editorStore.selectModule(index);
            }}
        >
            {isHidden ? (
                <div className="module-mask">
                    <IconEyeClosed style={{ fontSize: 25 }} />
                </div>
            ) : null}
            <div className="h5-editor-module-preview">
                {Preview ? (
                    <ModuleErrorBoundary label={moduleData.type} resetKey={moduleData.moduleIndex}>
                        <Suspense fallback={<div className="h5-editor-preview-loading">{t('模块加载中…')}</div>}>
                            <Preview data={moduleData.module} />
                        </Suspense>
                    </ModuleErrorBoundary>
                ) : (
                    <div className="h5-editor-preview-missing">「{t(moduleData.label || moduleData.type)}」{t('预览待移植')}</div>
                )}
            </div>
            {!preview ? (
                <>
                    {/* 拖拽手柄：dnd-kit 的 attributes/listeners 只挂在左侧模块名标签上，
                        不再摊满整个模块项——画布内点选/滚动手势不被拖拽占用；
                        标签可聚焦（attributes 带 tabIndex），配合 KeyboardSensor 支持键盘拖拽。 */}
                    <div
                        className="module-label module-label--handle"
                        title={t('拖拽排序')}
                        style={{ touchAction: "none" }}
                        {...attributes}
                        {...listeners}
                    >
                        <div className="label-name">
                            <span className="label-grip" aria-hidden="true">⋮⋮</span>
                            {t(moduleData.label || moduleData.type)}
                        </div>
                    </div>
                    {isActive ? <ModuleOperateBar moduleData={moduleData} index={index} /> : null}
                </>
            ) : null}
        </div>
    );
}

export default function Canvas({ preview = false }: { preview?: boolean }) {
    const { t } = useI18n();
    const s = useEditorState();
    /** 渲染模式：real = iframe 真实渲染（默认）/ compat = 旧 Preview 画布（兜底） */
    const [renderMode, setRenderMode] = useState<"real" | "compat">(() => getStoredRenderMode());
    const switchRenderMode = (mode: "real" | "compat") => {
        setRenderMode(mode);
        try {
            localStorage.setItem(RENDER_MODE_STORAGE_KEY, mode);
        } catch {
            /* 隐私模式等场景忽略 */
        }
    };
    const windowRef = useRef<HTMLDivElement>(null);
    const libDrag = useLibraryDragState();

    // 模块库拖入落点解析（兼容模式）：按旧画布模块项 DOM 位置算插入下标，
    // lineY 相对画布窗顶部（指示线渲染在画布窗内）。真实模式由 IframeCanvas 注册。
    useEffect(() => {
        if (renderMode !== "compat" || preview) return;
        return registerDropResolver((clientX, clientY) => {
            const win = windowRef.current;
            if (!win) return null;
            const wr = win.getBoundingClientRect();
            if (clientX < wr.left || clientX > wr.right || clientY < wr.top || clientY > wr.bottom) return null;
            const nodes = win.querySelectorAll(".gallery-list-ul .list-item");
            if (nodes.length === 0) return { insertIndex: 0, lineY: 0 };
            for (let i = 0; i < nodes.length; i++) {
                const r = nodes[i].getBoundingClientRect();
                if (clientY < r.top + r.height / 2) return { insertIndex: i, lineY: r.top - wr.top };
            }
            const last = nodes[nodes.length - 1].getBoundingClientRect();
            return { insertIndex: nodes.length, lineY: last.bottom - wr.top };
        });
    }, [renderMode, preview]);

    const sensors = useSensors(
        useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
        useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
    );

    const items = s.moduleList;

    const handleDragEnd = (event: DragEndEvent) => {
        const { active, over } = event;
        if (!over || active.id === over.id) return;
        const oldIndex = items.findIndex((m) => m.moduleIndex === active.id);
        const newIndex = items.findIndex((m) => m.moduleIndex === over.id);
        if (oldIndex === -1 || newIndex === -1) return;
        const validation: any = editorStore.moveModule(oldIndex, newIndex);
        if (validation !== true) {
            Toast.warning(t(String(validation?.reason || '')));
            return;
        }
    };

    const windowStyle = useMemo(() => pageBackgroundStyle(s.pageModule as Record<string, any>, s.pageType), [s.pageModule, s.pageType]);
    const showPageConfig = !preview && s.pageType !== 6 && s.pageType !== 8;

    return (
        <>
            {!preview ? (
                <div className="h5-editor-mode-switch" role="tablist" aria-label={t('画布渲染模式')}>
                    <button type="button" className={renderMode === "real" ? "active" : ""} onClick={() => switchRenderMode("real")}>
                        {t('真实渲染')}
                    </button>
                    <button type="button" className={renderMode === "compat" ? "active" : ""} onClick={() => switchRenderMode("compat")}>
                        {t('兼容渲染')}
                    </button>
                </div>
            ) : null}
            {/* 真实渲染分支保活（2026-11-19）：挂载后不随模式切换卸载，切到兼容
                模式时仅 display:none 隐藏——iframe 内的前台应用不销毁，切回真实
                模式即刻可用，不再整页重载嵌入页（此前一切换就全量重载，是编辑
                中途最明显的卡顿源之一）。落点解析归当前可见画布：隐藏时经
                active=false 让出注册。 */}
            <div
                className="decorate-page-window"
                style={renderMode === "real" ? windowStyle : { ...windowStyle, display: "none" }}
                aria-hidden={renderMode !== "real"}
            >
                <PageHeaderPreview preview={preview} />
                <IframeCanvas preview={preview} bare active={renderMode === "real"} />
            </div>
            {renderMode === "compat" ? (
            <div className="decorate-page-window" ref={windowRef} style={windowStyle}>
                <div className="theme-modules-warp">
                    <PageHeaderPreview preview={preview} />
                    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                        <SortableContext items={items.map((m) => m.moduleIndex)} strategy={verticalListSortingStrategy}>
                            <div className="gallery-list-ul">
                                {items.map((m, index) => (
                                    <ModuleItem key={m.moduleIndex} moduleData={m} index={index} preview={preview} />
                                ))}
                                {items.length === 0 && <div className="h5-editor-canvas-empty">{t('请在左侧模块库点击添加模块')}</div>}
                            </div>
                        </SortableContext>
                    </DndContext>
                </div>
                {libDrag.active && libDrag.lineY !== null ? (
                    <div className="h5-editor-insert-line" style={{ top: libDrag.lineY }} />
                ) : null}
            </div>
            ) : null}
            {showPageConfig ? (
                <div className="decorate-page-config-warp">
                    <div className={`page-config-item${s.editMode === "page" ? " current" : ""}`} onClick={() => editorStore.selectPage()}>
                        <span className="page-config-icon">
                            <IconSetting />
                        </span>
                        {t('页面设置')}
                    </div>
                </div>
            ) : null}
        </>
    );
}
