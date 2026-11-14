/**
 * 新人专享 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, TextField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function NewcomerComboSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="头部">
                <SwitchField data={data} onChange={onChange} field="showHeader" label="显示头部" />
                <TextField data={data} onChange={onChange} field="headerTitle" label="主标题" />
                <TextField data={data} onChange={onChange} field="headerSubTitle" label="副标题" />
                <SinglePicField data={data} onChange={onChange} field="containerBgImage" label="容器背景图" />
                <ColorField data={data} onChange={onChange} field="boxGradientStart" label="渐变起始色" />
                <ColorField data={data} onChange={onChange} field="boxGradientEnd" label="渐变结束色" />
                <ColorField data={data} onChange={onChange} field="contentBgColor" label="内容背景色" />
            </SettingGroup>
            <SettingGroup title="一重礼 · 商品">
                <SwitchField data={data} onChange={onChange} field="showSection1" label="显示一重礼" />
                <TextField data={data} onChange={onChange} field="section1Badge" label="角标文字" />
                <TextField data={data} onChange={onChange} field="section1Title" label="区块标题" />
                <ProductsSourceField data={data} onChange={onChange} />
                <NumberField data={data} onChange={onChange} field="productPercent" label="卡片宽度" min={15} max={80} suffix="%" />
                <NumberField data={data} onChange={onChange} field="productItemGap" label="卡片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="productImgRadius" label="图片圆角" max={30} suffix="px" />
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" />
                <ColorField data={data} onChange={onChange} field="productPriceColor" label="价格颜色" />
                <ColorField data={data} onChange={onChange} field="productTitleColor" label="商品名颜色" />
            </SettingGroup>
            <SettingGroup title="二重礼 · 优惠券">
                <SwitchField data={data} onChange={onChange} field="showSection2" label="显示二重礼" />
                <TextField data={data} onChange={onChange} field="section2Badge" label="角标文字" />
                <TextField data={data} onChange={onChange} field="section2Title" label="区块标题" />
                <NumberField data={data} onChange={onChange} field="couponPerRow" label="每行券数" min={1} max={5} />
                <ColorField data={data} onChange={onChange} field="couponAmountColor" label="券面额颜色" />
                <ColorField data={data} onChange={onChange} field="couponConditionColor" label="券门槛颜色" />
                <ColorField data={data} onChange={onChange} field="couponBgGradientStart" label="券背景渐变起" />
                <ColorField data={data} onChange={onChange} field="couponBgGradientEnd" label="券背景渐变止" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
