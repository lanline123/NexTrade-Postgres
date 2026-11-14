/**
 * 限时特惠 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, SelectField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function LimitedSpecialSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="容器">
                <SinglePicField data={data} onChange={onChange} field="containerBgImage" label="背景图片" />
                <ColorField data={data} onChange={onChange} field="boxGradientStart" label="渐变起始色" />
                <ColorField data={data} onChange={onChange} field="boxGradientEnd" label="渐变结束色" />
                <NumberField data={data} onChange={onChange} field="productAreaLeft" label="商品区左偏移" max={80} suffix="%" />
                <NumberField data={data} onChange={onChange} field="productAreaPaddingY" label="商品区上下边距" max={30} suffix="px" />
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="商品卡片">
                <NumberField data={data} onChange={onChange} field="productPercent" label="卡片宽度" min={15} max={80} suffix="%" />
                <NumberField data={data} onChange={onChange} field="productItemGap" label="卡片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="productItemPadding" label="卡片内边距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="productImgRadius" label="图片圆角" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="cardRadius" label="卡片圆角" max={30} suffix="px" />
                <SelectField data={data} onChange={onChange} field="textAlign" label="文字对齐" options={[{ value: "left", label: "居左" }, { value: "center", label: "居中" }, { value: "right", label: "居右" }]} />
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" />
                <SwitchField data={data} onChange={onChange} field="showSales" label="显示销量" />
                <ColorField data={data} onChange={onChange} field="cardBgColor" label="卡片背景色" />
                <ColorField data={data} onChange={onChange} field="productTitleColor" label="商品名颜色" />
                <ColorField data={data} onChange={onChange} field="productPriceColor" label="价格颜色" />
                <ColorField data={data} onChange={onChange} field="productMarketPriceColor" label="划线价颜色" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
