/**
 * 混合商品展示 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, SinglePicField, RadioRowField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function MixedProductDisplaySettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const groups: Array<Record<string, any>> = data?.productGroups || [];
    const setGroup = (i: number, p: Record<string, any>) => onChange({ productGroups: groups.map((g, idx) => (idx === i ? { ...g, ...p } : g)) });
    const mc = data?.moduleColor ?? {}; const setMc = (p: Record<string, any>) => onChange({ moduleColor: { ...mc, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品组">
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {groups.map((g, i) => (
                        <div key={i} style={{ display: "flex", gap: 6 }}>
                            <input value={g.title || ""} placeholder="组标题" onChange={(e) => setGroup(i, { title: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                            <input value={g.subTitle || ""} placeholder="副标题" onChange={(e) => setGroup(i, { subTitle: e.target.value })} style={{ flex: 1, height: 28, border: "1px solid var(--semi-color-border)", borderRadius: 4, padding: "0 6px" }} />
                        </div>
                    ))}
                </div>
                <SinglePicField data={data} onChange={onChange} field="rightPic" label="右侧图片" />
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="倒计时与样式">
                <SwitchField data={data} onChange={onChange} field="showCountdown" label="显示倒计时" />
                <RadioRowField data={data} onChange={onChange} field="countdownType" label="倒计时类型" options={[{ value: "dailyCycle", label: "每日循环" }, { value: "specifyTime", label: "指定时间" }]} />
                <SwitchField data={data} onChange={onChange} field="showProductPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="crossedOutPrice" label="显示划线价" />
                <NumberField data={data} onChange={onChange} field="productRadius" label="商品圆角" max={30} suffix="px" />
                <ColorField data={mc} onChange={setMc} field="titleColor" label="标题颜色" />
                <ColorField data={mc} onChange={setMc} field="priceColor" label="价格颜色" />
                <ColorField data={mc} onChange={setMc} field="sellingPointColor" label="卖点颜色" />
            </SettingGroup>
        </div>
    );
}
