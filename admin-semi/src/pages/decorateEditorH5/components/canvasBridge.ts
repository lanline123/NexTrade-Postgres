/**
 * 画布桥接层（2026-11-18 Phase 2：iframe 真实渲染 + 模块库拖入）
 *
 * 三件事：
 * 1. iframe 嵌入常量：前台 decoratePreview 页地址与来源（postMessage 严格校验 origin）。
 * 2. 落点解析注册表：当前画布（真实渲染 = iframe 覆盖层布局数据 / 兼容渲染 = 旧画布
 *    DOM 位置）注册一个 (clientX, clientY) → 插入下标 的解析器，模块库拖拽据此
 *    判断落点；lineY 为插入指示线在「该模式画线容器」内的纵坐标（真实模式 = 覆盖
 *    层内、兼容模式 = 画布窗内），由各自画布组件消费本文件的拖拽状态渲染。
 * 3. 模块库拖拽控制器：pointer 事件实现（长按 220ms 或移动 >8px 进入拖拽态），
 *    拖拽影子跟随指针；松手时若解析出插入下标则回调 onDrop，否则静默取消。
 *    拖拽一旦激活，随后的 click（点击添加）会被抑制一次，避免拖完又被点击重复添加。
 */
import { useSyncExternalStore } from "react";

/** 前台来源（编辑器在 admin.recy.cc，前台在 battery.recy.cc，跨源 postMessage） */
export const EMBED_ORIGIN = "https://battery.recy.cc";
/** 前台装修嵌入预览页（Phase 1，uniapp-react pages/decoratePreview） */
export const EMBED_URL = `${EMBED_ORIGIN}/mobile/pages/decoratePreview/index?embed=1`;

/** 前台回传的单模块布局（index = moduleList 下标，top/height 为嵌入页内容坐标） */
export interface EmbedLayoutItem {
    index: number;
    top: number;
    height: number;
}

export interface EmbedLayout {
    items: EmbedLayoutItem[];
    totalHeight: number;
}

// ==================== 落点解析注册表 ====================

export interface DropResolution {
    /** 在 moduleList 中插入到该下标之前（0..length） */
    insertIndex: number;
    /** 插入指示线纵坐标（相对于当前模式画线容器的顶部） */
    lineY: number;
}

type DropResolver = (clientX: number, clientY: number) => DropResolution | null;

let dropResolver: DropResolver | null = null;

export function registerDropResolver(fn: DropResolver | null) {
    dropResolver = fn;
}

export function resolveDrop(clientX: number, clientY: number): DropResolution | null {
    return dropResolver ? dropResolver(clientX, clientY) : null;
}

// ==================== 模块库拖拽状态（供画布渲染插入线） ====================

export interface LibraryDragState {
    active: boolean;
    label: string;
    insertIndex: number | null;
    lineY: number | null;
}

let dragState: LibraryDragState = { active: false, label: "", insertIndex: null, lineY: null };
const dragListeners = new Set<() => void>();

function setDragState(next: Partial<LibraryDragState>) {
    dragState = { ...dragState, ...next };
    dragListeners.forEach((l) => l());
}

export function useLibraryDragState(): LibraryDragState {
    return useSyncExternalStore(
        (cb) => {
            dragListeners.add(cb);
            return () => {
                dragListeners.delete(cb);
            };
        },
        () => dragState,
        () => dragState
    );
}

// ==================== 点击抑制（拖拽激活后吞掉紧随的 click 添加） ====================

let suppressUntil = 0;

function suppressAddClick() {
    suppressUntil = Date.now() + 400;
}

export function consumeAddClickSuppression(): boolean {
    if (Date.now() < suppressUntil) {
        suppressUntil = 0;
        return true;
    }
    return false;
}

// ==================== 模块库拖拽控制器 ====================

export interface LibraryDragStart {
    /** 拖拽影子上显示的模块名 */
    label: string;
    startX: number;
    startY: number;
    /** 松手落在画布上时回调（insertIndex 已由落点解析得出） */
    onDrop: (insertIndex: number) => void;
}

const ACTIVATION_DISTANCE = 8;
const PRESS_DELAY_MS = 220;

export function beginLibraryDrag({ label, startX, startY, onDrop }: LibraryDragStart) {
    let activated = false;
    let lastX = startX;
    let lastY = startY;

    const ghost = document.createElement("div");
    ghost.className = "h5-editor-lib-drag-ghost";
    ghost.textContent = label;
    ghost.style.display = "none";
    document.body.appendChild(ghost);

    const moveGhost = () => {
        ghost.style.left = `${lastX}px`;
        ghost.style.top = `${lastY}px`;
    };

    const activate = () => {
        if (activated) return;
        activated = true;
        ghost.style.display = "flex";
        moveGhost();
        setDragState({ active: true, label, insertIndex: null, lineY: null });
    };

    const pressTimer = window.setTimeout(activate, PRESS_DELAY_MS);

    const cleanup = () => {
        window.clearTimeout(pressTimer);
        window.removeEventListener("pointermove", onMove, true);
        window.removeEventListener("pointerup", onUp, true);
        window.removeEventListener("pointercancel", onUp, true);
        window.removeEventListener("keydown", onKey, true);
        ghost.remove();
        if (activated) setDragState({ active: false, label: "", insertIndex: null, lineY: null });
    };

    const onMove = (e: PointerEvent) => {
        lastX = e.clientX;
        lastY = e.clientY;
        if (!activated) {
            if (Math.hypot(lastX - startX, lastY - startY) > ACTIVATION_DISTANCE) {
                window.clearTimeout(pressTimer);
                activate();
            }
            return;
        }
        moveGhost();
        const res = resolveDrop(lastX, lastY);
        setDragState({ insertIndex: res ? res.insertIndex : null, lineY: res ? res.lineY : null });
    };

    const onUp = (e: PointerEvent) => {
        const wasActivated = activated;
        cleanup();
        if (!wasActivated) return; // 普通点击：交给 LibraryPanel 的 onClick 添加
        suppressAddClick();
        const res = resolveDrop(e.clientX, e.clientY);
        if (res) onDrop(res.insertIndex);
    };

    const onKey = (e: KeyboardEvent) => {
        if (e.key !== "Escape") return;
        const wasActivated = activated;
        cleanup();
        if (wasActivated) suppressAddClick();
    };

    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerup", onUp, true);
    window.addEventListener("pointercancel", onUp, true);
    window.addEventListener("keydown", onKey, true);
}
