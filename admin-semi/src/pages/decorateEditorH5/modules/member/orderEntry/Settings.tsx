/**
 * 我的订单 设置（对齐 view/admin orderEntry/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：布局样式（切换即整体替换 entryList 图标集 + 角标背景回默认，
 * 同原版 watch 联动）→ 模块标题（仅样式一，maxlength 10）→ 订单入口列表 PicList
 * （showText、showLink=false；原版 showAddButton=false，我方无此门控，见摘要）；
 * 扩展：角标背景色/角标文字色（重置回当前样式默认）+ CommonFrameEdit 全量。
 */
import { SettingGroup, TextField, SegmentedRadio, ColorField, DecDivider, PicListField, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { orderEntryStyle1List, orderEntryStyle2List, orderEntryBadgeBg } from "./index";

export default function OrderEntrySettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const layoutStyle = data?.layoutStyle ?? 1;
    const handleChange = (patch: Record<string, any>) => {
        if (patch.layoutStyle && patch.layoutStyle !== layoutStyle) {
            const next = patch.layoutStyle;
            onChange({
                ...patch,
                entryList: (next === 2 ? orderEntryStyle2List : orderEntryStyle1List).map((i) => ({ ...i })),
                badgeBgColor: orderEntryBadgeBg(next)
            });
            return;
        }
        onChange(patch);
    };
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <RadioRowField
                            data={data}
                            onChange={handleChange}
                            field="layoutStyle"
                            label="布局样式"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" }
                            ]}
                        />
                    </SettingGroup>
                    <DecDivider />
                    {layoutStyle === 1 ? (
                        <>
                            <SettingGroup>
                                <TextField data={data} onChange={onChange} field="title" label="模块标题" placeholder="我的订单" />
                            </SettingGroup>
                            <DecDivider />
                        </>
                    ) : null}
                    <PicListField data={data} onChange={onChange} field="entryList" label="订单入口列表" showText showLink={false} showAddButton={false} />
                </div>
                <div>
                    <SettingGroup>
                        <ColorField data={data} onChange={onChange} field="badgeBgColor" label="角标背景色" resetValue={orderEntryBadgeBg(layoutStyle)} />
                        <ColorField data={data} onChange={onChange} field="badgeTextColor" label="角标文字色" resetValue="#ffffff" />
                    </SettingGroup>
                    <FrameSettingsField data={data} onChange={onChange} />
                </div>
            </SettingsTabs>
        </div>
    );
}
