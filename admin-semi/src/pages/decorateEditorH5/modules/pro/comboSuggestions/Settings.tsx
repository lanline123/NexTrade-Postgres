/**
 * 组合建议 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function ComboSuggestionsSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const groups: Array<Record<string, any>> = data?.comboGroups || [];
    const setGroup = (i: number, p: Record<string, any>) => onChange({ comboGroups: groups.map((g, idx) => (idx === i ? { ...g, ...p } : g)) });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="搭配组">
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {groups.map((g, i) => (
                        <div key={i} style={{ display: "flex", gap: 6 }}>
                            <input value={g.title || ""} placeholder="组标题" onChange={(e) => setGroup(i, { title: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                            <input value={g.subTitle || ""} placeholder="副标题" onChange={(e) => setGroup(i, { subTitle: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                        </div>
                    ))}
                </div>
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="样式设置">
                <SwitchField data={data} onChange={onChange} field="autoPlay" label="自动轮播" />
                <NumberField data={data} onChange={onChange} field="interval" label="轮播间隔" min={1000} max={10000} step={500} suffix="ms" />
                <NumberField data={data} onChange={onChange} field="groupRadius" label="组圆角" max={30} suffix="px" />
                <SwitchField data={data} onChange={onChange} field="groupShadow" label="组阴影" />
                <ColorField data={data} onChange={onChange} field="groupTitleColor" label="组标题颜色" />
                <NumberField data={data} onChange={onChange} field="imageRadius" label="图片圆角" max={30} suffix="px" />
                <ColorField data={data} onChange={onChange} field="gradientColorA" label="渐变色A" />
                <ColorField data={data} onChange={onChange} field="gradientColorB" label="渐变色B" />
                <ColorField data={data} onChange={onChange} field="regularPriceColor" label="划线价颜色" />
                <ColorField data={data} onChange={onChange} field="regularPriceBgColor" label="划线价背景" />
            </SettingGroup>
        </div>
    );
}
