/**
 * 手机画布（移植自 view/admin editor/components/CanvasViewport.vue + ModuleRenderer.vue）
 *
 * 壳层对齐原版（editor/Index.vue + src/css/decorate.less）：
 * - 画布窗 .decorate-page-window：375px、margin 50px auto、阴影 0 0 14px rgba(0,0,0,.1)、无圆角
 * - 选中/悬停描边由 .module-ad-con:before 承担（2px solid #155bd4 / 1px dashed，见 decorate.css）
 * - 模块名标签 .module-label：画布左侧外 −100px 白底 chip（带右箭头）
 * - 操作条 .module-operate：右侧外 −40px 38px 白底竖排，上移/下移/显隐/复制/删除（fixed 隐藏后三项），hover 出 .opt-tip
 * - 隐藏模块：.module-mask rgba(255,255,255,.6) 遮罩 + 居中闭眼图标
 * - 画布首项：页面头模拟件（原版 MobilePageHeaderPreview），沉浸式/头部风格与页面设置联动
 * - 悬浮「页面设置」按钮 .decorate-page-config-warp（fixed right 400px / top 110px）
 * - dnd-kit 纵向拖拽排序（约束校验失败弹提示并回弹）
 * 预览态（preview=true）隐藏全部编辑外壳，只留画布内容。
 */
import { Suspense, useMemo, lazy } from "react";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Toast } from "@douyinfe/semi-ui";
import { IconChevronDown, IconChevronUp, IconCopy, IconDelete, IconEyeClosed, IconEyeOpened, IconSetting } from "@douyinfe/semi-icons";
import { editorStore, useEditorState } from "../store/editorStore";
import { getModuleDefinition } from "../contract/moduleRegistry";
import ModuleErrorBoundary from "./ModuleErrorBoundary";
import type { DecorateModuleBase } from "../contract/types";

import { useI18n } from '../../../i18n';
/** 预览组件缓存（moduleIndex 无关，按 type@url 缓存） */
const previewCache = new Map<string, React.ComponentType<any>>();

function getLazyPreview(type: string, url?: string): React.ComponentType<any> | null {
    const key = `${type}@${url ?? ""}`;
    if (!previewCache.has(key)) {
        const def = getModuleDefinition(type, url);
        if (!def) return null;
        previewCache.set(key, lazy(def.preview as any));
    }
    return previewCache.get(key)!;
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

function tryMove(from: number, to: number, t: (key: string) => string) {
    if (to < 0) return;
    const state = editorStore.getState();
    if (to >= state.moduleList.length) return;
    const result: any = editorStore.moveModule(from, to);
    if (result !== true) Toast.warning(t(String(result?.reason || '')));
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

    const operateItems: Array<{ key: string; tip: string; icon: React.ReactNode; onClick: () => void; hidden?: boolean }> = [
        { key: "up", tip: t('上移'), icon: <IconChevronUp />, onClick: () => tryMove(index, index - 1, t) },
        { key: "down", tip: t('下移'), icon: <IconChevronDown />, onClick: () => tryMove(index, index + 1, t) },
        {
            key: "visibility",
            tip: isHidden ? t('显示') : t('隐藏'),
            icon: isHidden ? <IconEyeClosed /> : <IconEyeOpened />,
            onClick: () => editorStore.toggleModuleVisibility(index),
            hidden: !!moduleData.fixed
        },
        { key: "copy", tip: t('复制'), icon: <IconCopy />, onClick: () => editorStore.copyModule(index), hidden: !!moduleData.fixed },
        { key: "delete", tip: t('删除'), icon: <IconDelete />, onClick: () => editorStore.removeModule(index), hidden: !!moduleData.fixed }
    ];

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={`list-item modules-item${isActive && !preview ? " module-item-active" : ""}`}
            onClick={() => {
                if (!preview) editorStore.selectModule(index);
            }}
            {...attributes}
            {...(preview ? {} : listeners)}
        >
            {isHidden ? (
                <div className="module-mask">
                    <IconEyeClosed style={{ fontSize: 25 }} />
                </div>
            ) : null}
            <div className="h5-editor-module-preview">
                {Preview ? (
                    <ModuleErrorBoundary label={moduleData.type}>
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
                    <div className="module-label">
                        <div className="label-name">{t(moduleData.label || moduleData.type)}</div>
                    </div>
                    {isActive ? (
                        <div className="module-operate" onClick={(e) => e.stopPropagation()}>
                            {operateItems
                                .filter((item) => !item.hidden)
                                .map((item) => (
                                    <div
                                        key={item.key}
                                        className="module-operate-item"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            item.onClick();
                                        }}
                                    >
                                        {item.icon}
                                        <div className="opt-tip">{item.tip}</div>
                                    </div>
                                ))}
                        </div>
                    ) : null}
                </>
            ) : null}
        </div>
    );
}

export default function Canvas({ preview = false }: { preview?: boolean }) {
    const { t } = useI18n();
    const s = useEditorState();
    const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));

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
            <div className="decorate-page-window" style={windowStyle}>
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
            </div>
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
