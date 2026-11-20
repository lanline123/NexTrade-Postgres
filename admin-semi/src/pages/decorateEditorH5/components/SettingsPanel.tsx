/**
 * 设置面板（移植自 view/admin editor/components/SettingPanel.vue）
 *
 * 壳层（2026-11-17 标准编辑器 Step 2）：统一走公共 DecSettingsPanel
 * （components/decorate；灰底 + 白底标题区 + 内容白卡，样式唯一标准在
 * editor-shell.css 的 .dec-settings-scope 段），本文件只负责内容分发。
 * 未选中模块时直接显示页面设置（原版无空态）；pageType 6/8 无页面设置（原版 SettingPanel 门控）。
 * 页面设置字段集对齐 src/modules/src/commonPage/Edit.vue（CommonPageEdit）。
 * editMode=module：懒加载当前模块的设置组件（协议：props = { data, onChange, moduleIndex }）。
 */
import { Suspense, useMemo } from "react";
import { Input, Switch } from "@douyinfe/semi-ui";
import DecSettingsPanel from "../../../components/decorate/DecSettingsPanel";
import { editorStore, useEditorState } from "../store/editorStore";
import { getLazyModuleComponent } from "../contract/lazyModule";
import { getModuleDefinition } from "../contract/moduleRegistry";
import ModuleErrorBoundary from "./ModuleErrorBoundary";
import { ColorField, PicSelectField, SegmentedButtonRadio, SegmentedButtonRadioRowField, DecDivider } from "../shared/settingWidgets";
import type { PageModule } from "../contract/types";

import { useI18n } from '../../../i18n';
/** 设置组件懒加载（缓存/失败逐出/部署切换自愈在 contract/lazyModule 统一处理） */
function getLazySettings(type: string, url?: string): React.ComponentType<any> | null {
    return getLazyModuleComponent("settings", type, url);
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
            {/* 页面设置以 PC 编辑器 PageSettings 为准：分段按钮（type="button"），非模块字段的圆点口径 */}
            <SegmentedButtonRadioRowField
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
                    <SegmentedButtonRadio
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
                    <SegmentedButtonRadio
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
    // 标题区说明：原版各模块 Settings 标题下自带一句说明，我方移植时漏传（壳层 DecSettingsPanel 早已支持 desc）；
    // 口径对齐 PC 编辑器——说明挂在模块注册定义（ModuleDefinition.desc）上，仅声明了的模块显示。
    const headerDesc = isModuleMode && activeModule ? getModuleDefinition(activeModule.type, activeModule.url)?.desc : undefined;

    return (
        <DecSettingsPanel title={t(headerTitle)} desc={headerDesc ? t(headerDesc) : undefined} className="h5-editor-settings">
            {isModuleMode && activeModule && SettingsComp ? (
                <ModuleErrorBoundary label={activeModule.type} resetKey={activeModule.moduleIndex}>
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
        </DecSettingsPanel>
    );
}
