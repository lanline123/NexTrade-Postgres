/**
 * 商品滑动 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, SelectField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function ProductSlideSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="主题卡片">
                <SelectField data={data} onChange={onChange} field="cardStyle" label="卡片风格" options={[{ value: "transparent", label: "透明" }, { value: "solid", label: "实色" }]} />
                <NumberField data={data} onChange={onChange} field="cardRadius" label="卡片圆角" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="productPercent" label="卡片宽度" min={20} max={90} suffix="%" />
                <NumberField data={data} onChange={onChange} field="productItemGap" label="卡片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="layerGap" label="标题与图片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="productImgRadius" label="图片圆角" max={30} suffix="px" />
                <ColorField data={data} onChange={onChange} field="cardTitleColor" label="卡片标题颜色" />
                <ColorField data={data} onChange={onChange} field="innerBgColor" label="内层背景色" />
                <SinglePicField data={data} onChange={onChange} field="moduleBgImage" label="模块背景图" />
            </SettingGroup>
            <SettingGroup title="展示设置">
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" />
                <SwitchField data={data} onChange={onChange} field="showSales" label="显示销量" />
                <SwitchField data={data} onChange={onChange} field="showBrief" label="显示卖点" />
                <SwitchField data={data} onChange={onChange} field="showSlidePeek" label="露出下一张" />
                <NumberField data={data} onChange={onChange} field="priceFontSize" label="价格字号" min={11} max={22} suffix="px" />
                <ColorField data={data} onChange={onChange} field="productPriceColor" label="价格颜色" />
                <ColorField data={data} onChange={onChange} field="badgeColor" label="角标颜色" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
