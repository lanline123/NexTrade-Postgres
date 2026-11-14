/**
 * 移动端登录页 设置（字段对齐 view/admin mobileLogin/Settings.vue）
 *
 * 分组标题用原版 .dec-edit-section-title 形态（SettingGroup variant="section"）：
 * 「颜色」5 色（重置取默认值：固定色回默认色、linkColor/buttonColor 回空跟随主题；
 * 注：types 中的 subTextColor/dividerColor 原版 Settings 同样未提供编辑项，仅 Preview 有 fallback）；
 * .dec-divider-line 分隔后「边距」组：contentPaddingX（max 60）/ contentPaddingTop（max 100）滑杆。
 */
import { SettingGroup, ColorField, RangeField, DecDivider } from "../../../shared/settingWidgets";
import { defaultMobileLogin } from "../../../contract/sharedDefaults";

const COLOR_FIELDS: Array<{ field: string; label: string }> = [
    { field: "titleColor", label: "标题颜色" },
    { field: "inputTextColor", label: "输入框文字颜色" },
    { field: "placeholderColor", label: "输入框提示文字颜色" },
    { field: "linkColor", label: "链接文字颜色（立即注册、协议）" },
    { field: "buttonColor", label: "登录按钮颜色" }
];

export default function MobileLoginSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="颜色" variant="section">
                {COLOR_FIELDS.map((item) => (
                    <ColorField
                        key={item.field}
                        data={data}
                        onChange={onChange}
                        field={item.field}
                        label={item.label}
                        resetValue={(defaultMobileLogin as Record<string, any>)[item.field] ?? ""}
                    />
                ))}
            </SettingGroup>
            <DecDivider />
            <SettingGroup title="边距" variant="section">
                <RangeField data={data} onChange={onChange} field="contentPaddingX" label="内容左右边距(px)" max={60} />
                <RangeField data={data} onChange={onChange} field="contentPaddingTop" label="内容顶部间距(px)" max={100} />
            </SettingGroup>
        </div>
    );
}
