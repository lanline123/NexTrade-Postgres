/**
 * 装修编辑器状态管理（React 版）
 *
 * 从 view/admin store/decorateEditor.ts（pinia）移植，移动端（decorateType=1）口径：
 * - getSaveData 整体透传：不做字段剔除（契约红线）
 * - 不做 moduleIndex 去重（契约红线）
 * - initDecorate / restoreFromDraft 内置旧编辑器的加载侧归一化链
 * - 固定模块（fixed）不可删除 / 复制 / 隐藏
 * - 会话级撤销/重做（我方新增，原版与旧移植均无）：全部文档变更经本 store 单一
 *   漏斗，在变更前对整页文档 {pageModule, moduleList} 做快照入历史栈；高频变更
 *   （滑杆/连续输入/颜色拖选）按「动作+目标+字段」合并键在时间窗内只记一步；
 *   撤销/重做后的脏标记以与保存点快照的比对为准（回到保存点即回落为未脏）
 *
 * 实现方式：vanilla external store + useSyncExternalStore（不引入额外依赖）。
 */

import { useSyncExternalStore } from "react";
import type { DecorateData, DecorateModuleBase, EditMode, ModuleValidationResult, PageModule } from "../contract/types";
import { DecorateType, type DecorateTypeValue } from "../contract/constants";
import {
    getModulePlacement,
    isBottomModule,
    isExclusiveModule,
    isTopModule
} from "../contract/moduleRegistry";
import { ensureFixedModules, updateModuleListWithUrls } from "../contract/normalize";

interface EditorState {
    decorateId: string | number;
    decorateType: DecorateTypeValue;
    /** 页面类型（移动端）：1=首页 2=会员中心 3=活动页 4=分类中心 5=登录 6=搜索 8=商品详情 */
    pageType: number;
    decorateTitle: string;
    pageModule: PageModule;
    moduleList: DecorateModuleBase[];
    activeModuleIndex: number | null;
    editMode: EditMode;
    hasDraft: boolean;
    isDirty: boolean;
    /** 撤销/重做可用性（历史栈派生，供顶栏按钮禁用态） */
    canUndo: boolean;
    canRedo: boolean;
    localeId: string | number;
    parentId: string | number;
    isDefault: boolean;
    /** 页面所属店铺（详情接口顶层 shopId；店铺模板页才有值）：供真实渲染 iframe 下发给嵌入页拉真实店铺详情 */
    shopId: number;
    /**
     * 预览样例商品 id（商品详情页编辑器指定，2026-11-21 方案二）：仅编辑会话
     * 内的瞬态视图状态，随 decorate-data 下发给真实渲染预览选样例商品；
     * 不属于装修数据——不进 getSaveData、不置 isDirty。0 = 默认样例。
     */
    previewProductId: number;
}

let state: EditorState = {
    decorateId: "",
    decorateType: 1,
    pageType: 1,
    decorateTitle: "",
    pageModule: {},
    moduleList: [],
    activeModuleIndex: null,
    editMode: null,
    hasDraft: false,
    isDirty: false,
    canUndo: false,
    canRedo: false,
    localeId: 0,
    parentId: 0,
    isDefault: true,
    shopId: 0,
    previewProductId: 0
};

const listeners = new Set<() => void>();

function set(next: Partial<EditorState>) {
    state = { ...state, ...next };
    listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
    listeners.add(listener);
    return () => listeners.delete(listener);
}

function getState() {
    return state;
}

/** 深拷贝（默认值/模块数据均为纯 JSON 结构） */
const cloneDeep = <T>(obj: T): T => JSON.parse(JSON.stringify(obj));

// ==================== 会话级撤销/重做（我方新增） ====================

/** 历史快照：与 getSaveData 同形状的整页文档 */
interface DocSnapshot {
    pageModule: PageModule;
    moduleList: DecorateModuleBase[];
}

/** 历史栈上限：整页快照每份约数十 KB，100 步足够覆盖一次编辑会话且不撑内存 */
const HISTORY_LIMIT = 100;
/** 高频变更合并窗口（毫秒）：滑杆拖动 / 连续输入 / 颜色拖选在窗口内同目标同字段只记一步 */
const HISTORY_COALESCE_MS = 800;

let historyPast: DocSnapshot[] = [];
let historyFuture: DocSnapshot[] = [];
/**
 * 保存点快照（JSON 串）：initDecorate / restoreFromDraft / markAsSaved 时刷新。
 * 撤销/重做后的脏标记以当前文档与它的比对为准——回到保存点即回落未脏，
 * 不绕过既有未保存退出拦截。
 */
let savedDocJson = "";
/** 上一步的合并键与时刻：同键且在窗口内 → 不再压栈（栈顶已是该高频流开始前的状态） */
let lastCoalesceKey: string | null = null;
let lastCoalesceAt = 0;

function docSnapshot(): DocSnapshot {
    return { pageModule: cloneDeep(state.pageModule), moduleList: cloneDeep(state.moduleList) };
}

function docJsonOf(doc: DocSnapshot): string {
    return JSON.stringify({ pageModule: doc.pageModule, moduleList: doc.moduleList });
}

function currentDocJson(): string {
    return docJsonOf({ pageModule: state.pageModule, moduleList: state.moduleList });
}

function historyFlags(): { canUndo: boolean; canRedo: boolean } {
    return { canUndo: historyPast.length > 0, canRedo: historyFuture.length > 0 };
}

/** 基线切换（进入编辑器 / 草稿恢复 / 重置）：清空历史，并以当前文档为保存点 */
function resetHistoryBaseline() {
    historyPast = [];
    historyFuture = [];
    lastCoalesceKey = null;
    lastCoalesceAt = 0;
    savedDocJson = currentDocJson();
}

/** 保存成功后推进保存点；历史保留（可继续撤销跨越保存点），合并键复位（跨保存点不合并） */
function advanceSavedBaseline() {
    savedDocJson = currentDocJson();
    lastCoalesceKey = null;
    lastCoalesceAt = 0;
}

/**
 * 变更前记录：把当前文档压入历史栈并清空重做栈。
 * 带 coalesceKey 且与上一步同键、在合并窗口内时不再压栈、只刷新时刻——
 * 持续拖动/输入始终并作一步，停顿超过窗口再继续则另起一步。
 * 与栈顶内容相同的快照不重复压栈（无实际变化的调用不占历史步数）。
 */
function recordHistory(coalesceKey?: string) {
    historyFuture = [];
    const now = Date.now();
    if (coalesceKey && coalesceKey === lastCoalesceKey && now - lastCoalesceAt <= HISTORY_COALESCE_MS) {
        lastCoalesceAt = now;
        return;
    }
    const snap = docSnapshot();
    const top = historyPast[historyPast.length - 1];
    if (!top || docJsonOf(top) !== docJsonOf(snap)) {
        historyPast.push(snap);
        if (historyPast.length > HISTORY_LIMIT) historyPast.shift();
    }
    lastCoalesceKey = coalesceKey ?? null;
    lastCoalesceAt = now;
}

/** 把历史快照写回 store（撤销/重做共用）：选中模块越界时收敛，脏标记按保存点比对 */
function applyHistorySnapshot(snap: DocSnapshot) {
    const list = cloneDeep(snap.moduleList);
    let activeModuleIndex = state.activeModuleIndex;
    let editMode = state.editMode;
    if (activeModuleIndex !== null && activeModuleIndex >= list.length) {
        activeModuleIndex = list.length > 0 ? list.length - 1 : null;
        if (activeModuleIndex === null) editMode = null;
    }
    set({
        pageModule: cloneDeep(snap.pageModule),
        moduleList: list,
        activeModuleIndex,
        editMode,
        isDirty: docJsonOf(snap) !== savedDocJson,
        ...historyFlags()
    });
}

// ==================== 模块约束校验 ====================

function topModuleCount(list: DecorateModuleBase[]) {
    return list.filter((m) => isTopModule(m.type)).length;
}

function bottomModuleCount(list: DecorateModuleBase[]) {
    return list.filter((m) => isBottomModule(m.type)).length;
}

function hasExclusiveModule(list: DecorateModuleBase[]) {
    return list.some((m) => isExclusiveModule(m.type));
}

/** 检查是否可以添加模块 */
function canAddModule(type: string, targetIndex?: number): ModuleValidationResult {
    const list = state.moduleList;
    const placement = getModulePlacement(type);
    const isExclusive = isExclusiveModule(type);

    if (hasExclusiveModule(list)) {
        return { ok: false, reason: "页面已包含独占模块，不能添加其他模块" };
    }
    if (isExclusive && list.length > 0) {
        return { ok: false, reason: "该模块为独占模块，需要清空页面后才能添加" };
    }
    if (placement === "top") {
        const validIndex = topModuleCount(list);
        if (targetIndex !== undefined && targetIndex > validIndex) {
            return { ok: false, reason: "该模块必须置顶，不能添加到此位置" };
        }
    } else if (placement === "bottom") {
        const validIndex = list.length - bottomModuleCount(list);
        if (targetIndex !== undefined && targetIndex < validIndex) {
            return { ok: false, reason: "该模块必须置底，不能添加到此位置" };
        }
    } else {
        if (targetIndex !== undefined && targetIndex < topModuleCount(list)) {
            return { ok: false, reason: "不能在置顶模块之上添加其他模块" };
        }
        const bottomStart = list.length - bottomModuleCount(list);
        if (targetIndex !== undefined && targetIndex > bottomStart) {
            return { ok: false, reason: "不能在置底模块之下添加其他模块" };
        }
    }
    return { ok: true };
}

/** 检查是否可以移动模块 */
function canMoveModule(fromIndex: number, toIndex: number): ModuleValidationResult {
    const list = state.moduleList;
    if (fromIndex < 0 || fromIndex >= list.length || toIndex < 0 || toIndex >= list.length) {
        return { ok: false, reason: "无效的位置" };
    }
    if (fromIndex === toIndex) {
        return { ok: true };
    }
    const movingModule = list[fromIndex];
    if (!movingModule) {
        return { ok: false, reason: "未找到要移动的模块" };
    }
    const movingPlacement = getModulePlacement(movingModule.type);
    const topCount = topModuleCount(list);
    const bottomCount = bottomModuleCount(list);
    const bottomStart = list.length - bottomCount;

    if (movingPlacement === "top") {
        if (toIndex >= topCount) {
            return { ok: false, reason: "置顶模块不能移动到非置顶区域" };
        }
    } else if (movingPlacement === "bottom") {
        if (toIndex < bottomStart) {
            return { ok: false, reason: "置底模块不能移动到非置底区域" };
        }
    } else {
        if (toIndex < topCount) {
            return { ok: false, reason: "不能将模块移动到置顶模块之上" };
        }
        if (toIndex >= bottomStart && bottomCount > 0) {
            return { ok: false, reason: "不能将模块移动到置底模块之下" };
        }
    }
    return { ok: true };
}

/** 按置顶/置底约束计算有效插入位置 */
function getValidInsertIndex(type: string, preferredIndex?: number): number {
    const list = state.moduleList;
    const placement = getModulePlacement(type);
    if (placement === "top") {
        return topModuleCount(list);
    } else if (placement === "bottom") {
        return list.length - bottomModuleCount(list);
    } else {
        const minIndex = topModuleCount(list);
        const maxIndex = list.length - bottomModuleCount(list);
        if (preferredIndex === undefined) return maxIndex;
        return Math.max(minIndex, Math.min(preferredIndex, maxIndex));
    }
}

// ==================== 商品详情三风格并存保留（2026-11-21 方案二 ④ 升级） ====================

/**
 * 三风格并存模型（对标模板并存切换，根治换风格丢配置）：
 * - 生效风格的配置就是模块自身的 module 数据；其余两风格的最近配置以
 *   module.styleVariants = { A1?, A2?, A3? } 挂在生效模块上，随页面一并
 *   保存/加载（附加可选字段，前台与各预览面均按名取字段、不受影响）。
 * - 本编辑器切换风格 = 删除旧模块 + 新增新模块（独占模块约束），两步之间
 *   靠下方会话暂存衔接：删除详情模块时把三份配置收进暂存，新增详情模块
 *   时按目标风格恢复其留存配置（无留存则用新模块默认值），其余风格继续
 *   以 styleVariants 留存。共享字段与风格专属字段都不丢。
 * - 暂存只活在编辑会话内：initDecorate / restoreFromDraft / applyTemplate /
 *   reset 时清空，避免跨页面串味。
 */
const PD_STYLE_BY_TYPE: Record<string, "A1" | "A2" | "A3"> = {
    productDetailA1: "A1",
    productDetailA2: "A2",
    productDetailA3: "A3"
};

let pdVariantsStash: Record<string, Record<string, any>> | null = null;

/** 删除详情模块时收获三份风格配置到会话暂存 */
function harvestPdVariants(moduleType: string, moduleData: Record<string, any> | undefined) {
    const style = PD_STYLE_BY_TYPE[moduleType];
    if (!style) return;
    const data = cloneDeep(moduleData ?? {}) as Record<string, any>;
    const variants: Record<string, Record<string, any>> = { ...(data.styleVariants ?? {}) };
    delete data.styleVariants;
    delete variants[style];
    variants[style] = data;
    pdVariantsStash = variants;
}

// ==================== store 动作 ====================

export const editorStore = {
    subscribe,
    getState,

    // -------------------- 初始化 --------------------

    /** 设置预览样例商品（瞬态视图状态：不置 isDirty、不进保存数据） */
    setPreviewProductId(id: number) {
        set({ previewProductId: id > 0 ? Math.floor(id) : 0 });
    },

    /** 初始化装修数据（详情加载 / 从其他页面复制 / 模板导入共用），内置 url 回填 + 固定模块注入 */
    initDecorate(data: {
        decorateId: string | number;
        decorateType: DecorateTypeValue;
        pageType?: number;
        decorateTitle?: string;
        pageModule?: PageModule;
        moduleList?: DecorateModuleBase[];
        hasDraft?: boolean;
        localeId?: string | number;
        parentId?: string | number;
        isDefault?: boolean;
        shopId?: number;
    }) {
        const pageModule = data.pageModule ? cloneDeep(data.pageModule) : {};
        const list = normalizeProductDetailStyles(updateModuleListWithUrls(cloneDeep(data.moduleList ?? [])));
        ensureFixedModules(list, data.pageType ?? 1);
        pdVariantsStash = null;
        set({
            decorateId: data.decorateId,
            decorateType: data.decorateType,
            pageType: data.pageType ?? 1,
            decorateTitle: data.decorateTitle ?? "",
            pageModule,
            moduleList: list,
            hasDraft: data.hasDraft ?? false,
            localeId: data.localeId ?? 0,
            parentId: data.parentId ?? 0,
            isDefault: data.isDefault ?? true,
            shopId: data.shopId ?? 0,
            previewProductId: 0,
            activeModuleIndex: null,
            editMode: null,
            isDirty: false,
            canUndo: false,
            canRedo: false
        });
        // 新页面基线：历史清空，加载内容即保存点
        resetHistoryBaseline();
    },

    /** 从草稿恢复：只注入固定模块，不回填 url（草稿保存时已带 url） */
    restoreFromDraft(draftData: DecorateData) {
        const list = normalizeProductDetailStyles(cloneDeep(draftData.moduleList ?? []));
        ensureFixedModules(list, state.pageType);
        pdVariantsStash = null;
        set({
            pageModule: draftData.pageModule ? cloneDeep(draftData.pageModule) : state.pageModule,
            moduleList: list,
            hasDraft: false,
            activeModuleIndex: null,
            editMode: null,
            canUndo: false,
            canRedo: false
        });
        // 草稿是已落库的保存形态：以草稿内容为新基线（历史清空、保存点=草稿）
        resetHistoryBaseline();
    },

    /** 应用模板（url 回填 + 固定模块注入，pageModule 合并而非替换）；整页替换可撤销 */
    applyTemplate(json: { moduleList?: DecorateModuleBase[]; pageModule?: PageModule }) {
        recordHistory();
        const list = normalizeProductDetailStyles(updateModuleListWithUrls(cloneDeep(json.moduleList ?? [])));
        ensureFixedModules(list, state.pageType);
        pdVariantsStash = null;
        set({
            moduleList: list,
            pageModule: { ...state.pageModule, ...(json.pageModule ?? {}) },
            isDirty: true,
            ...historyFlags()
        });
    },

    // -------------------- 选择 --------------------

    selectModule(index: number) {
        if (index >= 0 && index < state.moduleList.length) {
            set({ activeModuleIndex: index, editMode: "module" });
        }
    },

    selectPage() {
        set({ activeModuleIndex: null, editMode: "page" });
    },

    clearSelection() {
        set({ activeModuleIndex: null, editMode: null });
    },

    // -------------------- 模块操作 --------------------

    /**
     * 添加模块
     * @returns 成功返回 moduleIndex，失败返回校验结果
     */
    addModule(moduleData: Omit<DecorateModuleBase, "moduleIndex">, index?: number, skipValidation = false): number | ModuleValidationResult {
        if (!skipValidation) {
            const validation = canAddModule(moduleData.type, index);
            if (!validation.ok) return validation;
        }
        // 三风格并存：新增详情模块时恢复该风格的留存配置（暂存有则以留存
        // 覆盖新默认值，新字段仍取默认值兜底），其余风格转挂 styleVariants。
        const targetStyle = PD_STYLE_BY_TYPE[moduleData.type];
        if (targetStyle && pdVariantsStash) {
            const variants = { ...pdVariantsStash };
            const saved = variants[targetStyle];
            delete variants[targetStyle];
            const restored: Record<string, any> = { ...cloneDeep(moduleData.module ?? {}), ...(saved ? cloneDeep(saved) : {}) };
            restored.decorateStyle = targetStyle;
            if (Object.keys(variants).length > 0) restored.styleVariants = variants;
            else delete restored.styleVariants;
            moduleData = { ...moduleData, module: restored };
            pdVariantsStash = null;
        }
        recordHistory();
        const list = [...state.moduleList];
        const validIndex = getValidInsertIndex(moduleData.type, index);
        const newModule = {
            ...moduleData,
            moduleIndex: Date.now(),
            isShow: moduleData.isShow ?? true
        } as DecorateModuleBase;
        list.splice(validIndex, 0, newModule);
        set({ moduleList: list, activeModuleIndex: validIndex, editMode: "module", isDirty: true, ...historyFlags() });
        return newModule.moduleIndex;
    },

    /** 更新指定模块数据（浅合并）；同模块同字段的高频变更（滑杆/输入/颜色）在窗口内合并为一步 */
    updateModule(index: number, data: Record<string, any>) {
        const list = [...state.moduleList];
        const target = list[index];
        if (target) {
            recordHistory(`upd:${index}:${Object.keys(data).sort().join(",")}`);
            list[index] = { ...target, module: { ...target.module, ...data } };
            set({ moduleList: list, isDirty: true, ...historyFlags() });
        }
    },

    /** 更新当前激活模块（浅合并） */
    updateActiveModule(data: Record<string, any>) {
        if (state.activeModuleIndex !== null) {
            editorStore.updateModule(state.activeModuleIndex, data);
        }
    },

    /** 替换当前激活模块的全部数据 */
    replaceActiveModuleData(data: Record<string, any>) {
        if (state.activeModuleIndex !== null) {
            const list = [...state.moduleList];
            const target = list[state.activeModuleIndex];
            if (target) {
                recordHistory();
                list[state.activeModuleIndex] = { ...target, module: data };
                set({ moduleList: list, isDirty: true, ...historyFlags() });
            }
        }
    },

    /** 更新嵌套字段（如 frame.boxPadding） */
    patchNestedField(parentKey: string, childKey: string, value: unknown) {
        if (state.activeModuleIndex !== null) {
            const list = [...state.moduleList];
            const current = list[state.activeModuleIndex];
            if (current?.module) {
                const parent = current.module[parentKey];
                if (parent && typeof parent === "object") {
                    recordHistory(`patch:${state.activeModuleIndex}:${parentKey}.${childKey}`);
                    list[state.activeModuleIndex] = {
                        ...current,
                        module: { ...current.module, [parentKey]: { ...parent, [childKey]: value } }
                    };
                    set({ moduleList: list, isDirty: true, ...historyFlags() });
                }
            }
        }
    },

    /** 更新页面设置；同字段高频变更在窗口内合并为一步 */
    updatePageModule(data: Partial<PageModule>) {
        recordHistory(`page:${Object.keys(data).sort().join(",")}`);
        set({ pageModule: { ...state.pageModule, ...data }, isDirty: true, ...historyFlags() });
    },

    /** 删除模块（固定模块不可删除） */
    removeModule(index: number) {
        if (index >= 0 && index < state.moduleList.length) {
            if (state.moduleList[index]?.fixed) return;
            // 三风格并存：删详情模块前先收获三份风格配置（见 pdVariantsStash）
            const removed = state.moduleList[index];
            if (removed) harvestPdVariants(removed.type, removed.module as Record<string, any> | undefined);
            const list = state.moduleList.filter((_, i) => i !== index);
            let activeModuleIndex = state.activeModuleIndex;
            let editMode = state.editMode;
            if (list.length === 0) {
                activeModuleIndex = null;
                editMode = null;
            } else if (activeModuleIndex !== null) {
                if (activeModuleIndex >= list.length) {
                    activeModuleIndex = list.length - 1;
                } else if (activeModuleIndex > index) {
                    activeModuleIndex = activeModuleIndex - 1;
                }
            }
            recordHistory();
            set({ moduleList: list, activeModuleIndex, editMode, isDirty: true, ...historyFlags() });
        }
    },

    /** 复制模块（固定模块不可复制） */
    copyModule(index: number) {
        const source = state.moduleList[index];
        if (source && !source.fixed) {
            recordHistory();
            const copied = cloneDeep(source);
            copied.moduleIndex = Date.now();
            const list = [...state.moduleList];
            list.splice(index + 1, 0, copied);
            set({ moduleList: list, activeModuleIndex: index + 1, isDirty: true, ...historyFlags() });
        }
    },

    /**
     * 移动模块
     * @returns 成功返回 true，失败返回校验结果
     */
    moveModule(fromIndex: number, toIndex: number, skipValidation = false): true | ModuleValidationResult {
        if (fromIndex === toIndex) return true;
        if (!skipValidation) {
            const validation = canMoveModule(fromIndex, toIndex);
            if (!validation.ok) return validation;
        }
        if (fromIndex >= 0 && fromIndex < state.moduleList.length && toIndex >= 0 && toIndex < state.moduleList.length) {
            const list = [...state.moduleList];
            const [moved] = list.splice(fromIndex, 1);
            if (moved) {
                recordHistory();
                list.splice(toIndex, 0, moved);
                set({ moduleList: list, activeModuleIndex: toIndex, isDirty: true, ...historyFlags() });
            }
        }
        return true;
    },

    moveUp(index: number): true | ModuleValidationResult {
        if (index <= 0) return true;
        return editorStore.moveModule(index, index - 1);
    },

    moveDown(index: number): true | ModuleValidationResult {
        if (index >= state.moduleList.length - 1) return true;
        return editorStore.moveModule(index, index + 1);
    },

    /** 切换模块显隐（固定模块不可隐藏） */
    toggleModuleVisibility(index: number) {
        const target = state.moduleList[index];
        if (target && !target.fixed) {
            recordHistory();
            const list = [...state.moduleList];
            list[index] = { ...target, isShow: !target.isShow };
            set({ moduleList: list, isDirty: true, ...historyFlags() });
        }
    },

    // -------------------- 数据获取 --------------------

    /** 获取保存数据（契约出口）：整体透传，不做任何字段剔除 */
    getSaveData(): DecorateData {
        return {
            pageModule: cloneDeep(state.pageModule),
            moduleList: cloneDeep(state.moduleList)
        };
    },

    /** 获取指定模块数据 */
    getModuleData<T = Record<string, unknown>>(index: number): T | null {
        const target = state.moduleList[index];
        return target ? (target.module as T) : null;
    },

    // -------------------- 状态管理 --------------------

    /**
     * 撤销一步（会话级）：回到上一个文档快照。
     * 脏标记按保存点比对回落——撤回到保存点状态即未脏，退出拦截随之解除；
     * 撤过保存点则保持脏。风格并存暂存（pdVariantsStash）不随撤销动：
     * 快照内的模块数据自带 styleVariants，文档层恢复自洽。
     */
    undo() {
        const prev = historyPast.pop();
        if (!prev) return;
        historyFuture.push(docSnapshot());
        lastCoalesceKey = null;
        lastCoalesceAt = 0;
        applyHistorySnapshot(prev);
    },

    /** 重做一步（会话级）：回到下一个文档快照，脏标记同样按保存点比对 */
    redo() {
        const next = historyFuture.pop();
        if (!next) return;
        historyPast.push(docSnapshot());
        if (historyPast.length > HISTORY_LIMIT) historyPast.shift();
        lastCoalesceKey = null;
        lastCoalesceAt = 0;
        applyHistorySnapshot(next);
    },

    markAsSaved() {
        set({ isDirty: false, hasDraft: false });
        // 保存点推进到当前文档（历史保留：保存后仍可撤销跨越保存点）
        advanceSavedBaseline();
    },

    /** 语言子页首次保存后回填子记录 id（原版 syncLocaleChildId）：
     * 只换 id，不动文档内容/选中态/历史 */
    setDecorateId(decorateId: string | number) {
        set({ decorateId });
    },

    markHasDraft(value: boolean) {
        set({ hasDraft: value });
    },

    reset() {
        pdVariantsStash = null;
        set({
            decorateId: "",
            decorateType: 1,
            pageType: 1,
            decorateTitle: "",
            pageModule: {},
            moduleList: [],
            activeModuleIndex: null,
            editMode: null,
            hasDraft: false,
            isDirty: false,
            canUndo: false,
            canRedo: false,
            localeId: 0,
            parentId: 0,
            isDefault: true,
            shopId: 0,
            previewProductId: 0
        });
        resetHistoryBaseline();
    }
};

/** 商品详情装修风格契约升级：历史值按模块类型归一为 A1/A2/A3 */
function normalizeProductDetailStyles(list: DecorateModuleBase[]): DecorateModuleBase[] {
    const styleByType: Record<string, "A1" | "A2" | "A3"> = {
        productDetailA1: "A1",
        productDetailA2: "A2",
        productDetailA3: "A3"
    };
    list.forEach((item) => {
        const style = styleByType[item.type];
        if (style) {
            item.module ??= {};
            item.module.decorateStyle = style;
        }
    });
    return list;
}

// ==================== React 绑定 ====================

export function useEditorState(): EditorState {
    return useSyncExternalStore(subscribe, getState, getState);
}

/** 派生选择器（避免无关字段变化引起重渲染时使用） */
export function useEditorSelector<T>(selector: (s: EditorState) => T): T {
    return useSyncExternalStore(
        subscribe,
        () => selector(state),
        () => selector(state)
    );
}

export { DecorateType };

