/**
 * 分组列表编辑器（内容设置通用组件，2026-11-21 抽取，我方新增）
 *
 * 承接各模块「内容设置」里同形态的分组列表编辑：分组折叠卡（第 N 组）+
 * 组内字段编辑 + 添加一组 / 删除组 / 上移下移排序。此前 groupSliding /
 * categoryRec / comboSuggestions / productGrouping / tagNav 五个模块各自
 * 手写同一套 list/patchItem/moveItem 样板，现统一由本组件承接数据层，
 * 呈现层复用 ./styleEdits 的 GroupCardList 壳。
 *
 * 契约（与原版各 src/*GroupEdit.vue 的 v-model 数组写回同口径）：
 * - 列表挂在模块 data 的 field 字段（默认 groupList）下，增删改移均以
 *   onChange({ [field]: next }) 整体写回，不改组内对象的字段形状；
 * - 新组模板 emptyItem（深克隆后入列）或 createItem 工厂二选一；
 * - renderItem 收到 itemProps（{ data: item, onChange: patchItem }），
 *   与各字段组件（TextField/ColorField 等）的 { data, onChange } 协议一致。
 */
import type { ReactNode } from "react";
import { GroupCardList } from "./styleEdits";
import { cloneDeep } from "../contract/sharedDefaults";

export interface GroupListEditorProps {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
    /** 列表字段名（groupList / categoryList / comboGroups / productGroups 等） */
    field?: string;
    /** 添加按钮文案（原版 GroupListEdit 的 title 透传，如「添加一组」） */
    addText?: string;
    /** 新组模板对象（入列前深克隆）；与 createItem 二选一 */
    emptyItem?: Record<string, any>;
    /** 新组工厂（优先于 emptyItem） */
    createItem?: () => Record<string, any>;
    /** 是否允许删组：布尔，或按当前列表判定（如 tagNav 至少保留 1 组） */
    canRemove?: boolean | ((list: Array<Record<string, any>>) => boolean);
    /** 是否允许添加组（默认 true）；成员固定的列表（如详情区块排序）传 false */
    canAdd?: boolean;
    /** 卡头标题工厂（默认「第 N 组」）；成员自带名称时传，如 (item) => item.label */
    itemTitle?: (item: Record<string, any>, index: number) => string;
    collapsible?: boolean;
    renderItem: (
        item: Record<string, any>,
        index: number,
        itemProps: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }
    ) => ReactNode;
}

export default function GroupListEditor({
    data,
    onChange,
    field = "groupList",
    addText = "添加一组",
    emptyItem,
    createItem,
    canRemove = true,
    canAdd = true,
    itemTitle,
    collapsible = true,
    renderItem
}: GroupListEditorProps) {
    const list: Array<Record<string, any>> = Array.isArray(data?.[field]) ? data[field] : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ [field]: next });
    const patchItem = (index: number, patch: Record<string, any>) =>
        setList(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
    const moveItem = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= list.length) return;
        const next = [...list];
        const [it] = next.splice(index, 1);
        next.splice(target, 0, it);
        setList(next);
    };
    return (
        <GroupCardList
            items={list}
            addText={addText}
            collapsible={collapsible}
            canAdd={canAdd}
            itemTitle={itemTitle}
            canRemove={typeof canRemove === "function" ? canRemove(list) : canRemove}
            onAdd={() => setList([...list, createItem ? createItem() : cloneDeep(emptyItem ?? {})])}
            onRemove={(index) => setList(list.filter((_, i) => i !== index))}
            onMove={moveItem}
            renderItem={(item, index) =>
                renderItem(item, index, { data: item, onChange: (patch: Record<string, any>) => patchItem(index, patch) })
            }
        />
    );
}
