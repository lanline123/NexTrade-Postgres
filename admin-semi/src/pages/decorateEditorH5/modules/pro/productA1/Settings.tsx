/**
 * 商品A1 设置
 */
import { SettingGroup, RadioRowField, NumberField, ColorField, SwitchField, TitleSettingsField, FrameSettingsField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function ProductA1Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品">
                <RadioRowField data={data} onChange={onChange} field="style" label="选择模板" options={[{ value: 1, label: "一行一个" }, { value: 2, label: "一行两个" }, { value: 3, label: "一行三个" }, { value: 4, label: "详细列表" }]} />
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="展示设置">
                <SwitchField data={data} onChange={onChange} field="showName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showBrief" label="显示卖点" />
                <SwitchField data={data} onChange={onChange} field="showPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" />
                <SwitchField data={data} onChange={onChange} field="showProductLabel" label="显示商品标签" />
                <ColorField data={data} onChange={onChange} field="labelBackgroundColor" label="标签背景色" />
                <ColorField data={data} onChange={onChange} field="labelTextColor" label="标签文字色" />
                <NumberField data={data} onChange={onChange} field="labelFontSize" label="标签字号" min={9} max={20} suffix="px" />
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" />
                <NumberField data={data} onChange={onChange} field="goodsRadius" label="商品圆角" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="goodsPadding" label="商品间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="itemGap" label="卡片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxRadius" label="模块圆角" max={30} suffix="px" />
            </SettingGroup>
            <TitleSettingsField data={data} onChange={onChange} label="模块标题" />
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
