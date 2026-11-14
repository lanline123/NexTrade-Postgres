/**
 * 我的服务 设置（覆盖渲染端读取的 moduleTitle/layoutStyle/iconRadius/serviceList/frame）
 */
import { SettingGroup, TextField, RadioRowField, NumberField, FrameSettingsField } from "../../../shared/settingWidgets";

const inputStyle: React.CSSProperties = { height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px", flex: 1, minWidth: 0 };

/** 入口列表近似编辑：picTitle / picUrl / picLink.link 三列文本（后续统一接图片/链接选择器） */
function PicEntryRows({ list, onList }: { list: any[]; onList: (next: any[]) => void }) {
    const setRow = (index: number, patch: Record<string, any>) => onList(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
    return (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {list.map((item, index) => (
                <div key={index} style={{ display: "flex", gap: 6 }}>
                    <input value={item.picTitle || ""} placeholder="名称" onChange={(e) => setRow(index, { picTitle: e.target.value })} style={inputStyle} />
                    <input value={item.picUrl || ""} placeholder="图标URL" onChange={(e) => setRow(index, { picUrl: e.target.value })} style={inputStyle} />
                    <input
                        value={item.picLink?.link || ""}
                        placeholder="链接(空=默认)"
                        onChange={(e) => setRow(index, { picLink: { ...(item.picLink || {}), link: e.target.value } })}
                        style={inputStyle}
                    />
                </div>
            ))}
        </div>
    );
}

export default function MyServiceSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const serviceList: any[] = data?.serviceList || [];
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="服务">
                <TextField data={data} onChange={onChange} field="moduleTitle" label="模块标题" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="layoutStyle"
                    label="布局样式"
                    options={[
                        { value: 1, label: "列表" },
                        { value: 2, label: "宫格" }
                    ]}
                />
                <NumberField data={data} onChange={onChange} field="iconRadius" label="图标圆角" max={50} suffix="%" />
            </SettingGroup>
            <SettingGroup title="服务列表">
                <PicEntryRows list={serviceList} onList={(next) => onChange({ serviceList: next })} />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
