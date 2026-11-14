/**
 * 订单入口 V2 设置（覆盖渲染端读取的 title/showTitle/showMore/cardList/listItems/颜色/frame）
 */
import { SettingGroup, TextField, SwitchField, ColorField, FrameSettingsField } from "../../../shared/settingWidgets";

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

export default function OrderEntryV2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const cardList: any[] = data?.cardList || [];
    const listItems: any[] = data?.listItems || [];
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="标题">
                <TextField data={data} onChange={onChange} field="title" label="标题文字" />
                <SwitchField data={data} onChange={onChange} field="showTitle" label="显示标题" />
                <SwitchField data={data} onChange={onChange} field="showMore" label="显示查看更多" />
                <ColorField data={data} onChange={onChange} field="cardBackgroundColor" label="卡片区背景色" />
            </SettingGroup>
            <SettingGroup title="订单卡片列表">
                <PicEntryRows list={cardList} onList={(next) => onChange({ cardList: next })} />
            </SettingGroup>
            <SettingGroup title="下方列表项">
                <PicEntryRows list={listItems} onList={(next) => onChange({ listItems: next })} />
            </SettingGroup>
            <SettingGroup title="角标颜色">
                <ColorField data={data} onChange={onChange} field="badgeBgColor" label="角标背景色" />
                <ColorField data={data} onChange={onChange} field="badgeTextColor" label="角标文字色" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
