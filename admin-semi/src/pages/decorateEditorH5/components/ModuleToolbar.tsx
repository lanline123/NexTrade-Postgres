/**
 * 模块工具栏（移植自 view/admin editor/components/ModuleToolbar.vue）
 *
 * 结构：外壳 .theme-toolbar / .theme-menu-con（295px，见 decorate.css）内为
 * 上下文页签（Semi Tabs，按上下文映射为 6 类：基础模块/高级模块/会员模块/
 * 分类中心模块/商品详情/登录页模块）；每个页签内是标准模块库 LibraryPanel
 * （components/decorate；折叠分组 + 图标卡片三列网格，与 PC 编辑器同组件同
 * 样式，2026-11-17 标准编辑器 Step 2 接入）。本文件只负责数据映射与添加动作。
 * 点击添加（拖拽克隆添加未实现）；约束校验收敛到 store。
 */
import { useMemo } from "react";
import { Tabs, TabPane, Toast } from "@douyinfe/semi-ui";
import LibraryPanel, { type LibraryGroup } from "../../../components/decorate/LibraryPanel";
import { MODULE_ICONPARK } from "./moduleIcons";
import { getModulesByContext, isExclusiveModule, createModuleInstance } from "../contract/moduleRegistry";
import { getAvailableToolbarGroups, type ToolbarGroup } from "../contract/constants";
import { editorStore } from "../store/editorStore";
import type { ModuleDefinition } from "../contract/types";

import { useI18n } from '../../../i18n';
/** 页签归属（原版 TAB_OF_GROUP）：searchResult 并入基础模块页签 */
const TAB_OF_GROUP: Record<ToolbarGroup, string> = {
    basic: "toolList",
    other: "toolList",
    live: "toolList",
    searchResult: "toolList",
    advanced: "advancedList",
    member: "memberList",
    cateCenter: "cateCenterList",
    productDetail: "productDetailList",
    loginConfig: "loginList"
};

const TAB_LABELS: Record<string, string> = {
    toolList: "基础模块",
    advancedList: "高级模块",
    memberList: "会员模块",
    cateCenterList: "分类中心模块",
    productDetailList: "商品详情",
    loginList: "登录页模块"
};

const TAB_ORDER = ["toolList", "advancedList", "memberList", "cateCenterList", "productDetailList", "loginList"];

/** 折叠分组段头（原版 GROUP_LABELS）及段顺序 */
const GROUP_LABELS: Record<ToolbarGroup, string> = {
    basic: "基础模块",
    other: "其它模块",
    live: "直播模块",
    advanced: "高级模块",
    member: "会员模块",
    cateCenter: "分类中心模块",
    productDetail: "商品详情模块",
    searchResult: "搜索结果模块",
    loginConfig: "登录页模块"
};

const GROUP_ORDER: ToolbarGroup[] = ["basic", "other", "live", "advanced", "member", "cateCenter", "productDetail", "searchResult", "loginConfig"];

function groupByUrl(url: string): ToolbarGroup {
    switch (url) {
        case "mobile":
            return "basic";
        case "mobilePromotion":
            return "other";
        case "mobilePro":
            return "advanced";
        case "mobileMember":
            return "member";
        case "mobileCateCenter":
            return "cateCenter";
        case "mobileProductDetail":
            return "productDetail";
        case "mobileLogin":
            return "loginConfig";
        case "mobileSearch":
            return "searchResult";
        default:
            return "basic";
    }
}

export default function ModuleToolbar({ pageType, isShop }: { pageType: number; isShop?: boolean }) {
    const { t } = useI18n();
    const groups = useMemo(() => getAvailableToolbarGroups(1, pageType), [pageType]);
    const modules = useMemo(() => getModulesByContext(1, pageType, { isShop }), [pageType, isShop]);
    const defByKey = useMemo(() => {
        const map = new Map<string, ModuleDefinition>();
        for (const m of modules) map.set(`${m.type}@${m.url}`, m);
        return map;
    }, [modules]);

    /** tabKey -> group -> modules */
    const tabs = useMemo(() => {
        const byGroup = new Map<ToolbarGroup, ModuleDefinition[]>();
        for (const m of modules) {
            const group = (m.toolbarGroup as ToolbarGroup) ?? groupByUrl(m.url);
            if (!byGroup.has(group)) byGroup.set(group, []);
            byGroup.get(group)!.push(m);
        }
        const byTab = new Map<string, Array<{ group: ToolbarGroup; list: ModuleDefinition[] }>>();
        for (const group of GROUP_ORDER) {
            if (!groups.includes(group)) continue;
            const list = byGroup.get(group);
            if (!list || list.length === 0) continue;
            const tabKey = TAB_OF_GROUP[group];
            if (!byTab.has(tabKey)) byTab.set(tabKey, []);
            byTab.get(tabKey)!.push({ group, list });
        }
        return TAB_ORDER.filter((k) => byTab.has(k)).map((k) => ({ key: k, sections: byTab.get(k)! }));
    }, [modules, groups]);

    const handleAdd = (def: ModuleDefinition) => {
        if (isExclusiveModule(def.type) && editorStore.getState().moduleList.length > 0) {
            Toast.warning(t("该模块为独占模块，需要清空页面后才能添加"));
            return;
        }
        const instance = createModuleInstance(def.type, def.url);
        if (!instance) return;
        const result: any = editorStore.addModule(instance);
        if (result !== true && result && typeof result === "object" && "ok" in result && result.ok === false) {
            Toast.warning(t(String(result.reason || '')));
        }
    };

    /** 页签内分组 → 标准模块库数据（图标在此处实例化，添加时按 key 回查 def） */
    const toLibraryGroups = (sections: Array<{ group: ToolbarGroup; list: ModuleDefinition[] }>): LibraryGroup[] =>
        sections.map(({ group, list }) => ({
            key: group,
            title: t(GROUP_LABELS[group]),
            items: list.map((m) => {
                const ParkIcon = MODULE_ICONPARK[m.type];
                return {
                    key: `${m.type}@${m.url}`,
                    label: t(m.label),
                    title: t(m.label),
                    icon: ParkIcon ? (
                        <ParkIcon size={28} strokeWidth={3.5} theme="outline" />
                    ) : (
                        <span className="h5-editor-module-icon-fallback" dangerouslySetInnerHTML={{ __html: m.icon || "" }} />
                    )
                };
            })
        }));

    return (
        <div className="theme-toolbar">
            <div className="theme-menu-con show">
                <div className="theme-con-box">
                    <Tabs type="line" size="small" className="h5-editor-toolbar-tabs">
                        {tabs.map((tab) => (
                            <TabPane tab={t(TAB_LABELS[tab.key])} itemKey={tab.key} key={tab.key}>
                                <LibraryPanel
                                    flush
                                    groups={toLibraryGroups(tab.sections)}
                                    onAdd={(_groupKey, item) => {
                                        const def = defByKey.get(item.key);
                                        if (def) handleAdd(def);
                                    }}
                                />
                            </TabPane>
                        ))}
                    </Tabs>
                </div>
            </div>
        </div>
    );
}
