/**
 * 商品分组 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, TextField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function ProductGroupingSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const groups: Array<Record<string, any>> = data?.productGroups || [];
    const setGroup = (i: number, p: Record<string, any>) => onChange({ productGroups: groups.map((g, idx) => (idx === i ? { ...g, ...p } : g)) });
    const gt = data?.groupTitle ?? {}; const setGt = (p: Record<string, any>) => onChange({ groupTitle: { ...gt, ...p } });
    const pc = data?.productColor ?? {}; const setPc = (p: Record<string, any>) => onChange({ productColor: { ...pc, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="分组标签">
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {groups.map((g, i) => (
                        <div key={i} style={{ display: "flex", gap: 6 }}>
                            <input value={g.title || ""} placeholder="分组标题" onChange={(e) => setGroup(i, { title: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                            <input value={g.subTitle || ""} placeholder="副标题" onChange={(e) => setGroup(i, { subTitle: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                        </div>
                    ))}
                </div>
                <SwitchField data={gt} onChange={setGt} field="showTitle" label="显示分组标题" />
                <SwitchField data={gt} onChange={setGt} field="showSubTitle" label="显示副标题" />
                <ColorField data={gt} onChange={setGt} field="titleColor" label="标题颜色" />
                <ColorField data={gt} onChange={setGt} field="activeTitleColor" label="选中标题颜色" />
                <ColorField data={gt} onChange={setGt} field="activeSubTitleColor" label="选中副标题颜色" />
                <NumberField data={gt} onChange={setGt} field="titleFontSize" label="标题字号" min={12} max={22} suffix="px" />
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="商品卡片">
                <NumberField data={data} onChange={onChange} field="cardRadius" label="卡片圆角" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="cardMargin" label="卡片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="cardPadding" label="卡片内边距" max={30} suffix="px" />
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showProductPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="crossedOutPrice" label="显示划线价" />
                <SwitchField data={data} onChange={onChange} field="bottomMoreBtn" label="显示底部更多" />
                <TextField data={data} onChange={onChange} field="bottomMoreBtnText" label="更多按钮文字" />
                <ColorField data={pc} onChange={setPc} field="priceColor" label="价格颜色" />
                <ColorField data={pc} onChange={setPc} field="backgroundColor" label="卡片背景色" />
            </SettingGroup>
        </div>
    );
}
