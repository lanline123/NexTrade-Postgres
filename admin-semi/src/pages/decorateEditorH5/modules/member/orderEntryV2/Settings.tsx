/**
 * 我的订单2 设置（对齐 view/admin orderEntryV2/Settings.vue）
 *
 * 页签 内容设置/扩展设置。内容：模块标题（maxlength 10，placeholder 订单）→ 卡片入口 PicList →
 * 列表入口 PicList（均 showText、showLink=false；原版 showAddButton=false，我方无此门控，见摘要）；
 * 扩展：卡片背景色/角标背景色/角标文字色（带原版重置值）+ CommonFrameEdit 全量。
 */
import { SettingGroup, TextField, ColorField, DecDivider, PicListEditorField, FrameSettingsField } from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";

export default function OrderEntryV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    <SettingGroup>
                        <TextField data={data} onChange={onChange} field="title" label="模块标题" placeholder="订单" />
                    </SettingGroup>
                    <DecDivider />
                    <PicListEditorField data={data} onChange={onChange} field="cardList" label="卡片入口" showText showLink={false} showAddButton={false} />
                    <DecDivider />
                    <PicListEditorField data={data} onChange={onChange} field="listItems" label="列表入口" showText showLink={false} showAddButton={false} />
                </div>
                <div>
                    <SettingGroup>
                        <ColorField data={data} onChange={onChange} field="cardBackgroundColor" label="卡片背景色" resetValue="#f6f7fb" />
                        <ColorField data={data} onChange={onChange} field="badgeBgColor" label="角标背景色" resetValue="#ff4d4f" />
                        <ColorField data={data} onChange={onChange} field="badgeTextColor" label="角标文字色" resetValue="#ffffff" />
                    </SettingGroup>
                    <FrameSettingsField data={data} onChange={onChange} />
                </div>
            </SettingsTabs>
        </div>
    );
}
