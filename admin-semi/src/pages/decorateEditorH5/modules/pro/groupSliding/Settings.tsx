/**
 * 分组滑动 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function GroupSlidingSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const groups: Array<Record<string, any>> = data?.groupList || [];
    const setGroup = (i: number, p: Record<string, any>) => onChange({ groupList: groups.map((g, idx) => (idx === i ? { ...g, ...p } : g)) });
    const mc = data?.moduleColor ?? {}; const setMc = (p: Record<string, any>) => onChange({ moduleColor: { ...mc, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="分组列表">
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {groups.map((g, i) => (
                        <div key={i} style={{ display: "flex", gap: 6 }}>
                            <input value={g.title || ""} placeholder="分组标题" onChange={(e) => setGroup(i, { title: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                            <input value={g.subTitle || ""} placeholder="副标题" onChange={(e) => setGroup(i, { subTitle: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                        </div>
                    ))}
                </div>
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="样式设置">
                <NumberField data={data} onChange={onChange} field="groupWidth" label="卡片宽度" min={40} max={100} suffix="%" />
                <NumberField data={data} onChange={onChange} field="groupRadius" label="卡片圆角" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="productPicRadius" label="图片圆角" max={30} suffix="px" />
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showProductPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="showRanking" label="显示排名" />
                <SwitchField data={data} onChange={onChange} field="crossedOutPrice" label="显示划线价" />
                <ColorField data={mc} onChange={setMc} field="titleColor" label="标题颜色" />
                <ColorField data={mc} onChange={setMc} field="productPriceColor" label="价格颜色" />
            </SettingGroup>
        </div>
    );
}
