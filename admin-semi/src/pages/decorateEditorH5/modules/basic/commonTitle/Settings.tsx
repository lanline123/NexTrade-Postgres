/**
 * 通用标题 设置（字段直接位于模块数据根部，与渲染端 commonTitle 读取结构一致；
 * moreLink 渲染端经 urlFormat 解析，字符串 path 可直接透传，故用文本框填 path 占位）
 */
import { SettingGroup, SettingRow, SwitchField, RadioRowField, TextField, ColorField, NumberField, SinglePicField } from "../../../shared/settingWidgets";

export default function CommonTitleSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const moreLinkText = typeof data?.moreLink === "string" ? data.moreLink : "";
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="通用标题">
                <SwitchField data={data} onChange={onChange} field="showTitle" label="显示标题" />
                {Number(data?.showTitle) === 1 && (
                    <>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="titleStyle"
                            label="标题样式"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" },
                                { value: 3, label: "样式三" }
                            ]}
                        />
                        <TextField data={data} onChange={onChange} field="titleText" label="标题文字" />
                        <ColorField data={data} onChange={onChange} field="titleColor" label="标题颜色" />
                        <TextField data={data} onChange={onChange} field="descText" label="描述文字" />
                        <ColorField data={data} onChange={onChange} field="descColor" label="描述颜色" />
                        <ColorField data={data} onChange={onChange} field="titleBackground" label="标题背景色" />
                        <ColorField data={data} onChange={onChange} field="titleBackground2" label="背景渐变色" />
                        <NumberField data={data} onChange={onChange} field="titleRadius" label="标题圆角" max={50} suffix="px" />
                        <SinglePicField data={data} onChange={onChange} field="titleBackgroundPic" label="标题背景图" />
                        <SwitchField data={data} onChange={onChange} field="showMore" label="右侧显示更多" />
                        {Number(data?.showMore) === 1 && (
                            <>
                                <ColorField data={data} onChange={onChange} field="moreColor" label="更多颜色" />
                                <SettingRow label="更多链接">
                                    <input
                                        value={moreLinkText}
                                        placeholder="/pages/coupon/index"
                                        onChange={(e) => onChange({ moreLink: e.target.value })}
                                        style={{ width: 180, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }}
                                    />
                                </SettingRow>
                            </>
                        )}
                    </>
                )}
            </SettingGroup>
        </div>
    );
}
