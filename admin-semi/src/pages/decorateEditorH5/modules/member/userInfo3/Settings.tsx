/**
 * 会员信息3 设置（覆盖渲染端读取的全部配置字段；stats.items 的 key 为渲染端固定枚举，仅开放 label/show 编辑）
 */
import { SettingGroup, TextField, SwitchField, RadioRowField, NumberField, ColorField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

const inputStyle: React.CSSProperties = { height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px", flex: 1, minWidth: 0 };

export default function UserInfo3Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const obj = (key: string) => data?.[key] ?? {};
    const setObj = (key: string) => (patch: Record<string, any>) => onChange({ [key]: { ...obj(key), ...patch } });
    const stats = obj("stats");
    const statsItems: any[] = stats.items || [];
    const setStatItem = (index: number, patch: Record<string, any>) =>
        onChange({ stats: { ...stats, items: statsItems.map((it, i) => (i === index ? { ...it, ...patch } : it)) } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="布局">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="layoutStyle"
                    label="布局样式"
                    options={[
                        { value: 1, label: "弧形绿底" },
                        { value: 2, label: "浅色图标" },
                        { value: 3, label: "极简成长" },
                        { value: 4, label: "居中头像" }
                    ]}
                />
                <SwitchField data={data} onChange={onChange} field="showEditIcon" label="显示编辑图标" />
                <TextField data={data} onChange={onChange} field="nickname" label="昵称(演示)" />
                <TextField data={data} onChange={onChange} field="description" label="等级文案(演示)" />
            </SettingGroup>
            <SettingGroup title="背景渐变">
                <ColorField data={obj("gradient")} onChange={setObj("gradient")} field="colorA" label="渐变色A" />
                <ColorField data={obj("gradient")} onChange={setObj("gradient")} field="colorB" label="渐变色B" />
                <RadioRowField
                    data={obj("gradient")}
                    onChange={setObj("gradient")}
                    field="gradientType"
                    label="渐变方向"
                    options={[
                        { value: "leftRight", label: "左右" },
                        { value: "upDown", label: "上下" },
                        { value: "diagonal", label: "斜向" }
                    ]}
                />
                <ColorField data={data} onChange={onChange} field="bgColor" label="旧版背景色(兼容)" />
                <SwitchField data={data} onChange={onChange} field="backgroundImageShow" label="显示背景图" />
                <SinglePicField data={data} onChange={onChange} field="backgroundImage" label="背景图" />
            </SettingGroup>
            <SettingGroup title="快捷入口图标">
                <SwitchField data={obj("signIn")} onChange={setObj("signIn")} field="show" label="显示签到" />
                <TextField data={obj("signIn")} onChange={setObj("signIn")} field="title" label="签到文案" />
                <SinglePicField data={obj("signIn")} onChange={setObj("signIn")} field="icon" label="签到图标" />
                <SwitchField data={obj("settings")} onChange={setObj("settings")} field="show" label="显示设置" />
                <SinglePicField data={obj("settings")} onChange={setObj("settings")} field="icon" label="设置图标" />
                <SwitchField data={obj("notification")} onChange={setObj("notification")} field="show" label="显示通知" />
                <SinglePicField data={obj("notification")} onChange={setObj("notification")} field="icon" label="通知图标" />
            </SettingGroup>
            <SettingGroup title="统计区">
                <SwitchField data={stats} onChange={setObj("stats")} field="show" label="显示统计区" />
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {statsItems.map((it, i) => (
                        <div key={it.key || i} style={{ display: "flex", gap: 6, alignItems: "center" }}>
                            <span style={{ fontSize: 12, color: "#999", width: 52 }}>{it.key}</span>
                            <input value={it.label || ""} placeholder="标签" onChange={(e) => setStatItem(i, { label: e.target.value })} style={inputStyle} />
                            <input type="checkbox" checked={Number(it.show ?? 1) !== 0} onChange={(e) => setStatItem(i, { show: e.target.checked ? 1 : 0 })} />
                        </div>
                    ))}
                </div>
            </SettingGroup>
            <SettingGroup title="会员条">
                <SwitchField data={obj("memberBar")} onChange={setObj("memberBar")} field="show" label="显示会员条" />
                <TextField data={obj("memberBar")} onChange={setObj("memberBar")} field="levelName" label="等级名称" />
                <TextField data={obj("memberBar")} onChange={setObj("memberBar")} field="description" label="描述文案" />
                <TextField data={obj("memberBar")} onChange={setObj("memberBar")} field="buttonText" label="按钮文字" />
                <ColorField data={obj("memberBar")} onChange={setObj("memberBar")} field="backgroundColor" label="背景色A" />
                <ColorField data={obj("memberBar")} onChange={setObj("memberBar")} field="backgroundColorB" label="背景色B" />
                <NumberField data={obj("memberBar")} onChange={setObj("memberBar")} field="borderRadius" label="圆角" max={30} suffix="px" />
                <SinglePicField data={obj("memberBar")} onChange={setObj("memberBar")} field="waveImage" label="波纹图(样式4)" />
            </SettingGroup>
            <SettingGroup title="文字颜色">
                <ColorField data={obj("textColors")} onChange={setObj("textColors")} field="nameColor" label="昵称颜色" />
                <ColorField data={obj("textColors")} onChange={setObj("textColors")} field="subColor" label="副文案颜色" />
                <ColorField data={obj("textColors")} onChange={setObj("textColors")} field="statValueColor" label="统计数值色" />
                <ColorField data={obj("textColors")} onChange={setObj("textColors")} field="statLabelColor" label="统计标签色" />
            </SettingGroup>
            <SettingGroup title="底部弧形">
                <SwitchField data={obj("bottomArc")} onChange={setObj("bottomArc")} field="show" label="显示弧形(样式1)" />
                <NumberField data={obj("bottomArc")} onChange={setObj("bottomArc")} field="height" label="弧形高度" max={60} suffix="px" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
