/**
 * 我的钱包 V2 设置（items 的 key 为渲染端固定枚举不可改，仅开放 label 与 icon.picUrl 编辑）
 */
import { SettingGroup, FrameSettingsField } from "../../../shared/settingWidgets";

const inputStyle: React.CSSProperties = { height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px", flex: 1, minWidth: 0 };

export default function MyWalletV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const items: any[] = data?.items || [];
    const setItem = (index: number, patch: Record<string, any>) => onChange({ items: items.map((it, i) => (i === index ? { ...it, ...patch } : it)) });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="钱包格子">
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {items.map((item, index) => (
                        <div key={item.key || index} style={{ display: "flex", gap: 6, alignItems: "center" }}>
                            <span style={{ fontSize: 12, color: "#999", width: 52 }}>{item.key}</span>
                            <input value={item.label || ""} placeholder="标签" onChange={(e) => setItem(index, { label: e.target.value })} style={inputStyle} />
                            <input
                                value={item.icon?.picUrl || ""}
                                placeholder="图标URL"
                                onChange={(e) => setItem(index, { icon: { ...(item.icon || {}), picUrl: e.target.value } })}
                                style={inputStyle}
                            />
                        </div>
                    ))}
                </div>
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
