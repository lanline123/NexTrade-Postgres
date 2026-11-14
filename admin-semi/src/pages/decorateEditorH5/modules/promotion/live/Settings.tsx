/**
 * 直播 设置（字段覆盖渲染端 live/index.tsx 读取的可配置字段）
 *
 * selectedLiveList 行编辑字段与渲染端 LiveRoomState 对齐：roomId、name、
 * anchorName、feedsImg、liveStatus（101 直播中 / 102 未开始 / 103 已结束）。
 */
import { SettingGroup, RadioRowField, TextField, NumberField, FrameSettingsField } from "../../../shared/settingWidgets";

const inputStyle: React.CSSProperties = { height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px", fontSize: 12 };

export default function LiveSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const list: Array<Record<string, any>> = Array.isArray(data?.selectedLiveList) ? data.selectedLiveList : [];
    const setRow = (index: number, patch: Record<string, any>) => {
        onChange({ selectedLiveList: list.map((item, i) => (i === index ? { ...item, ...patch } : item)) });
    };
    const addRow = () =>
        onChange({
            selectedLiveList: [...list, { roomId: 0, name: "", anchorName: "", feedsImg: "", liveStatus: 102, productData: [] }]
        });
    const removeRow = (index: number) => onChange({ selectedLiveList: list.filter((_, i) => i !== index) });

    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="直播设置">
                <TextField data={data} onChange={onChange} field="titleText" label="标题文字" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="displayMode"
                    label="展示模式"
                    options={[
                        { value: "list", label: "列表" },
                        { value: "bigImage", label: "大图" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="sourceType"
                    label="数据来源"
                    options={[
                        { value: "auto", label: "自动获取" },
                        { value: "manual", label: "手动选择" }
                    ]}
                />
                {data?.sourceType !== "manual" && <NumberField data={data} onChange={onChange} field="autoLiveNum" label="自动获取数量" min={1} max={10} />}
            </SettingGroup>
            <SettingGroup title="手动选择直播间">
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {list.map((item, index) => (
                        <div key={index} style={{ display: "flex", flexDirection: "column", gap: 4, border: "1px solid #f0f0f0", borderRadius: 4, padding: 8 }}>
                            <div style={{ display: "flex", gap: 6 }}>
                                <input value={item.name || ""} placeholder="直播间名称" onChange={(e) => setRow(index, { name: e.target.value })} style={{ ...inputStyle, flex: 1 }} />
                                <input
                                    value={item.roomId ?? ""}
                                    placeholder="房间ID"
                                    onChange={(e) => setRow(index, { roomId: Number(e.target.value) || 0 })}
                                    style={{ ...inputStyle, width: 80 }}
                                />
                            </div>
                            <div style={{ display: "flex", gap: 6 }}>
                                <input value={item.anchorName || ""} placeholder="主播名" onChange={(e) => setRow(index, { anchorName: e.target.value })} style={{ ...inputStyle, flex: 1 }} />
                                <input
                                    value={item.liveStatus ?? 102}
                                    placeholder="状态码"
                                    onChange={(e) => setRow(index, { liveStatus: Number(e.target.value) || 0 })}
                                    style={{ ...inputStyle, width: 80 }}
                                />
                            </div>
                            <input value={item.feedsImg || ""} placeholder="封面图 URL（feedsImg）" onChange={(e) => setRow(index, { feedsImg: e.target.value })} style={inputStyle} />
                            <button
                                type="button"
                                onClick={() => removeRow(index)}
                                style={{ alignSelf: "flex-end", fontSize: 12, color: "var(--semi-color-danger)", background: "transparent", border: 0, cursor: "pointer" }}
                            >
                                删除
                            </button>
                        </div>
                    ))}
                    <button
                        type="button"
                        onClick={addRow}
                        style={{ height: 26, fontSize: 12, border: "1px solid var(--semi-color-border)", borderRadius: 4, background: "transparent", cursor: "pointer" }}
                    >
                        + 添加直播间
                    </button>
                </div>
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
