/**
 * 登录入口 设置（字段即 defaultMobileLogin 全部字段；链接色/按钮色留空 = 跟随主题色）
 */
import { SettingGroup, ColorField, NumberField } from "../../../shared/settingWidgets";

export default function MobileLoginSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="颜色">
                <ColorField data={data} onChange={onChange} field="titleColor" label="标题颜色" />
                <ColorField data={data} onChange={onChange} field="subTextColor" label="副文案颜色" />
                <ColorField data={data} onChange={onChange} field="inputTextColor" label="输入文字色" />
                <ColorField data={data} onChange={onChange} field="placeholderColor" label="占位文字色" />
                <ColorField data={data} onChange={onChange} field="linkColor" label="链接颜色(空=主题)" />
                <ColorField data={data} onChange={onChange} field="buttonColor" label="按钮颜色(空=主题)" />
                <ColorField data={data} onChange={onChange} field="dividerColor" label="分隔线颜色" />
            </SettingGroup>
            <SettingGroup title="边距">
                <NumberField data={data} onChange={onChange} field="contentPaddingX" label="左右边距" max={60} suffix="px" />
                <NumberField data={data} onChange={onChange} field="contentPaddingTop" label="顶部边距" max={100} suffix="px" />
            </SettingGroup>
        </div>
    );
}
