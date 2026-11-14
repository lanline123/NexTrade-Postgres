/**
 * 店铺订单卡 设置（渲染端暂无入口，字段先按契约做全，待渲染端补入口后直接生效）
 */
import { SettingGroup, NumberField, ColorField, TextField } from "../../../shared/settingWidgets";

export default function StoreOrderCardSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const items: Array<Record<string, any>> = data?.items || [];
    const setItem = (i: number, p: Record<string, any>) => onChange({ items: items.map((it, idx) => (idx === i ? { ...it, ...p } : it)) });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="订单卡">
                <div style={{ color: "#646a73", fontSize: 12, marginBottom: 8 }}>渲染端暂无入口，本模块不进工具栏，仅保证存量数据可编辑/预览。</div>
                <TextField data={data} onChange={onChange} field="title" label="卡片标题" />
                <NumberField data={data} onChange={onChange} field="showCount" label="展示状态数" min={1} max={5} />
                <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 8 }}>
                    {items.map((it, i) => (
                        <div key={i} style={{ display: "flex", gap: 6 }}>
                            <input value={it.icon || ""} placeholder="图标" onChange={(e) => setItem(i, { icon: e.target.value })} style={{ width: 60, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                            <input value={it.title || ""} placeholder="状态名" onChange={(e) => setItem(i, { title: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                        </div>
                    ))}
                </div>
            </SettingGroup>
            <SettingGroup title="样式设置">
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="卡片背景色" />
                <ColorField data={data} onChange={onChange} field="titleColor" label="标题颜色" />
                <ColorField data={data} onChange={onChange} field="itemColor" label="状态文字色" />
                <ColorField data={data} onChange={onChange} field="badgeColor" label="角标颜色" />
                <NumberField data={data} onChange={onChange} field="cardRadius" label="卡片圆角" max={30} suffix="px" />
            </SettingGroup>
        </div>
    );
}
