/**
 * 账户信息 设置（覆盖渲染端读取的 moduleTitle/accountList/frame；列表行含 picDesc 与 isShow 开关）
 */
import { SettingGroup, TextField, FrameSettingsField } from "../../../shared/settingWidgets";

const inputStyle: React.CSSProperties = { height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px", flex: 1, minWidth: 0 };

export default function AccountSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const accountList: any[] = data?.accountList || [];
    const setRow = (index: number, patch: Record<string, any>) => onChange({ accountList: accountList.map((it, i) => (i === index ? { ...it, ...patch } : it)) });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="账户">
                <TextField data={data} onChange={onChange} field="moduleTitle" label="模块标题" />
            </SettingGroup>
            <SettingGroup title="账户入口列表">
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {accountList.map((item, index) => (
                        <div key={index} style={{ display: "flex", gap: 6, alignItems: "center" }}>
                            <input value={item.picTitle || ""} placeholder="名称" onChange={(e) => setRow(index, { picTitle: e.target.value })} style={inputStyle} />
                            <input value={item.picDesc || ""} placeholder="描述" onChange={(e) => setRow(index, { picDesc: e.target.value })} style={inputStyle} />
                            <input value={item.picUrl || ""} placeholder="图标URL" onChange={(e) => setRow(index, { picUrl: e.target.value })} style={inputStyle} />
                            <input
                                value={item.picLink?.link || ""}
                                placeholder="链接(空=默认)"
                                onChange={(e) => setRow(index, { picLink: { ...(item.picLink || {}), link: e.target.value } })}
                                style={inputStyle}
                            />
                            <input type="checkbox" checked={Number(item.isShow ?? 1) !== 0} onChange={(e) => setRow(index, { isShow: e.target.checked ? 1 : 0 })} />
                        </div>
                    ))}
                </div>
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
