/**
 * 图文介绍 设置（对齐 view/admin mobilePro/imageTextComponent/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置（ModuleStyle type="customProduct" +
 * ContentStyle type="imageTextComponent"）。
 * 图片边框颜色仅在 picBorder > 0 时出现（原版门控）。
 */
import { TextArea } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, TextField, RangeField, ColorField, SinglePicField, RadioRowField, DecDivider } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, ContentStyleSettingsField } from "../../../shared/styleEdits";

const ALIGN_OPTIONS = [
    { value: "left", label: "左对齐" },
    { value: "center", label: "居中对齐" },
    { value: "right", label: "右对齐" },
];

export default function ImageTextComponentSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="海报设置：宽度不限">
                        <SinglePicField data={data} onChange={onChange} field="picUrl" label="" />
                    </SettingGroup>
                    <TextField data={data} onChange={onChange} field="title" label="标题文字" placeholder="请输入标题文字信息" />
                    <SettingRow label="介绍文字" block>
                        <TextArea
                            value={data?.description ?? ""}
                            rows={4}
                            placeholder="请输入介绍文字信息"
                            style={{ width: "100%" }}
                            onChange={(v: string) => onChange({ description: v })}
                        />
                    </SettingRow>
                    <SettingGroup title="参数设置">
                        <RangeField data={data} onChange={onChange} field="picBorder" label="图片边框" max={10} />
                        {Number(data?.picBorder) > 0 && (
                            <ColorField data={data} onChange={onChange} field="picBorderColor" label="图片边框颜色" resetValue="#eeeeee" />
                        )}
                        <RangeField data={data} onChange={onChange} field="picRadius" label="图片圆角半径" max={40} />
                        <DecDivider />
                        <RadioRowField data={data} onChange={onChange} field="titTextAlignment" label="标题对齐方式" options={ALIGN_OPTIONS} />
                        <RangeField data={data} onChange={onChange} field="titMarginBottom" label="标题介绍间距" max={50} />
                        <ColorField data={data} onChange={onChange} field="titColor" label="标题文字颜色" resetValue="#333333" />
                        <RadioRowField data={data} onChange={onChange} field="descTextAlignment" label="介绍对齐方式" options={ALIGN_OPTIONS} />
                        <ColorField data={data} onChange={onChange} field="descColor" label="介绍文字颜色" resetValue="#333333" />
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="customProduct" />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="imageTextComponent" />
                </>
            </SettingsTabs>
        </div>
    );
}
