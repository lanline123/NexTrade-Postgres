/**
 * 设置面板（移植自 view/admin editor/components/SettingPanel.vue）
 *
 * 壳层对齐原版：外壳 .decorate-edit-wrap（fixed 右侧 380px，见 decorate.css）；
 * 标题区 .dec-edit-title（padding 24px 16px、h3 18px/600 #323233）；内容区 .decorate-edit-con。
 * 未选中模块时直接显示页面设置（原版无空态）；pageType 6/8 无页面设置（原版 SettingPanel 门控）。
 * 页面设置字段集对齐 src/modules/src/commonPage/Edit.vue（CommonPageEdit）。
 * editMode=module：懒加载当前模块的设置组件（协议：props = { data, onChange, moduleIndex }）。
 */
import { Suspense, lazy, useMemo } from "react";
import { Input, Switch } from "@douyinfe/semi-ui";
import { editorStore, useEditorState } from "../store/editorStore";
import { getModuleDefinition } from "../contract/moduleRegistry";
import ModuleErrorBoundary from "./ModuleErrorBoundary";
import { ColorField, PicSelectField, RadioRowField, SegmentedRadio, DecDivider } from "../shared/settingWidgets";
import type { PageModule } from "../contract/types";

import { useI18n } from '../../../i18n';
const settingsCache = new Map<string, React.ComponentType<any>>();

function getLazySettings(type: string, url?: string): React.ComponentType<any> | null {
    const key = `${type}@${url ?? ""}`;
    if (!settingsCache.has(key)) {
        const def = getModuleDefinition(type, url);
        if (!def) return null;
        settingsCache.set(key, lazy(def.settings as any));
    }
    return settingsCache.get(key)!;
}

/** 页面设置（对齐 CommonPageEdit 字段顺序与门控；字段挂在 pageModule 上，保存路径不变） */
function PageSettings({ pageModule, pageType }: { pageModule: PageModule; pageType: number }) {
    const { t } = useI18n();
    const data = pageModule as Record<string, any>;
    const onChange = (patch: Record<string, any>) => editorStore.updatePageModule(patch);
    const fp = { data, onChange };
    const repeatLabels: Record<number, string> = { 1: "不平铺", 2: "平铺", 3: "纵向平铺", 4: "横向平铺" };
    const sizeLabels: Record<number, string> = { 1: "横向100%", 2: "纵向100%", 3: "100%填充" };

    return (
        <div className="h5-editor-settings-body">
            {/* 页面标题（原版 dec-edit-group-block + .title 标签形态） */}
            <div className="dec-edit-group dec-edit-group-block">
                <div className="dec-edit-group-title">
                    <div className="title">{t('页面标题')}</div>
                </div>
                <div className="dec-edit-group-con">
                    <Input
                        value={data.title ?? ""}
                        placeholder={t('请输入页面标题，可不填，默认为页面名称')}
                        onChange={(v: string) => onChange({ title: v })}
                    />
                </div>
            </div>
            {pageType === 2 ? (
                <div className="dec-edit-group">
                    <div className="dec-edit-group-title">
                        <div className="label">{t('沉浸式标题')}</div>
                    </div>
                    <div className="dec-edit-group-con">
                        <Switch checked={data.immersive === true || Number(data.immersive) === 1} onChange={(v: boolean) => onChange({ immersive: v })} />
                    </div>
                </div>
            ) : null}
            <RadioRowField
                {...fp}
                field="headerStyle"
                label="头部风格"
                options={[
                    { value: 1, label: "默认风格" },
                    { value: 2, label: "简约风格" }
                ]}
            />
            {/* 原版此项无「重置」链（showDecColorReset 未开），用 resetValue 同值实现等效无操作重置 */}
            <ColorField {...fp} field="titleBackgroundColor" label="标题栏背景颜色" resetValue={String(data.titleBackgroundColor ?? "")} />
            <ColorField {...fp} field="titleColor" label="标题栏文字颜色" resetValue={String(data.titleColor ?? "")} />
            <div className="dec-edit-group">
                <div className="dec-edit-group-desc" style={{ marginTop: 0 }}>
                    {t('提示：标题背景颜色和文字修改仅适用于微信小程序和原生/半原生APP。')}
                </div>
            </div>
            <DecDivider />
            <ColorField {...fp} field="backgroundColor" label="页面背景颜色" resetValue={String(data.backgroundColor ?? "")} />
            <PicSelectField {...fp} field="backgroundImage" label="背景图" />
            <div className="dec-edit-group dec-edit-group-block">
                <div className="dec-edit-group-title">
                    <div className="label">{t('背景图显示')}</div>
                    <div className="value">{t(repeatLabels[Number(data.backgroundRepeat ?? 1)] || "不平铺")}</div>
                </div>
                <div className="dec-edit-group-con">
                    <SegmentedRadio
                        value={Number(data.backgroundRepeat ?? 1)}
                        options={[
                            { value: 1, label: "不平铺" },
                            { value: 2, label: "平铺" },
                            { value: 3, label: "纵向平铺" },
                            { value: 4, label: "横向平铺" }
                        ]}
                        onChange={(v) => onChange({ backgroundRepeat: Number(v) })}
                    />
                </div>
            </div>
            <div className="dec-edit-group dec-edit-group-block">
                <div className="dec-edit-group-title">
                    <div className="label">{t('背景图填充大小')}</div>
                    <div className="value">{t(sizeLabels[Number(data.backgroundSize ?? 1)] || "横向100%")}</div>
                </div>
                <div className="dec-edit-group-con">
                    <SegmentedRadio
                        value={Number(data.backgroundSize ?? 1)}
                        options={[
                            { value: 1, label: "横向100%" },
                            { value: 2, label: "纵向100%" },
                            { value: 3, label: "100%填充" }
                        ]}
                        onChange={(v) => onChange({ backgroundSize: Number(v) })}
                    />
                </div>
            </div>
        </div>
    );
}

export default function SettingsPanel() {
    const { t } = useI18n();
    const s = useEditorState();
    const activeModule = s.activeModuleIndex !== null ? s.moduleList[s.activeModuleIndex] : null;
    const isModuleMode = s.editMode === "module" && !!activeModule;
    // 原版：pageType 6/8 不提供页面设置，未选中模块时面板为空内容区
    const showPageSettings = !isModuleMode && s.pageType !== 6 && s.pageType !== 8;

    const SettingsComp = useMemo(() => {
        if (!isModuleMode || !activeModule) return null;
        return getLazySettings(activeModule.type, activeModule.url);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [isModuleMode, activeModule?.type, activeModule?.url, activeModule?.moduleIndex]);

    const headerTitle = isModuleMode && activeModule ? activeModule.label || activeModule.type : "页面设置";

    return (
        <div className="decorate-edit-wrap h5-editor-settings">
            <div className="dec-edit-title">
                <h3>{t(headerTitle)}</h3>
            </div>
            <div className="decorate-edit-con">
                {isModuleMode && activeModule && SettingsComp ? (
                    <ModuleErrorBoundary label={activeModule.type}>
                        <Suspense fallback={<div className="h5-editor-preview-loading">{t('设置加载中…')}</div>}>
                            <SettingsComp
                                data={activeModule.module}
                                onChange={(patch: Record<string, any>) => editorStore.updateActiveModule(patch)}
                                moduleIndex={activeModule.moduleIndex}
                            />
                        </Suspense>
                    </ModuleErrorBoundary>
                ) : showPageSettings ? (
                    <PageSettings pageModule={s.pageModule} pageType={s.pageType} />
                ) : null}
            </div>
        </div>
    );
}
