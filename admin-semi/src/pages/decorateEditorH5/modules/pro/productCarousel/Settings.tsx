/**
 * 商品轮播 设置（productList 为逐项选商品的等价商品来源：每项可改图/标题/链接，此处覆盖卡片与展示字段）
 */
import { SettingGroup, NumberField, ColorField, SwitchField, TextField, SelectField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function ProductCarouselSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const items: any[] = data?.productList || [];
    const buy = data?.buyBtnStyle ?? {}; const setBuy = (p: Record<string, any>) => onChange({ buyBtnStyle: { ...buy, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品列表">
                <div style={{ color: "#646a73", fontSize: 12 }}>
                    当前共 {items.length} 个轮播商品（productList 逐项选商品，可改图/标题/链接；演示占位为 productId=0 的示例卡）。请在商品选择器中维护后回填 productList。
                </div>
                <SwitchField data={data} onChange={onChange} field="autoplay" label="自动轮播" />
                <NumberField data={data} onChange={onChange} field="cardWidthPercent" label="卡片宽度" min={40} max={100} suffix="%" />
                <NumberField data={data} onChange={onChange} field="itemGap" label="卡片间距" max={40} suffix="px" />
            </SettingGroup>
            <SettingGroup title="模块标题">
                <SwitchField data={data} onChange={onChange} field="showModuleTitle" label="显示模块标题" />
                <TextField data={data} onChange={onChange} field="moduleTitle" label="标题文字" />
                <ColorField data={data} onChange={onChange} field="moduleTitleColor" label="标题颜色" />
                <NumberField data={data} onChange={onChange} field="titleContentGap" label="标题与内容间距" max={60} suffix="px" />
                <SinglePicField data={data} onChange={onChange} field="moduleBgImage" label="模块背景图" />
            </SettingGroup>
            <SettingGroup title="卡片样式">
                <SelectField data={data} onChange={onChange} field="cardStyle" label="卡片风格" options={[{ value: "shadow", label: "阴影" }, { value: "flat", label: "扁平" }]} />
                <NumberField data={data} onChange={onChange} field="goodsRadius" label="卡片圆角" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="innerPadding" label="卡片内边距" max={40} suffix="px" />
                <SelectField data={data} onChange={onChange} field="textAlign" label="文字对齐" options={[{ value: "left", label: "居左" }, { value: "center", label: "居中" }, { value: "right", label: "居右" }]} />
                <SwitchField data={data} onChange={onChange} field="showName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showDesc" label="显示描述" />
                <SwitchField data={data} onChange={onChange} field="showPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" />
                <SwitchField data={data} onChange={onChange} field="showSales" label="显示销量" />
                <ColorField data={data} onChange={onChange} field="productTitleColor" label="商品名颜色" />
                <ColorField data={data} onChange={onChange} field="priceColor" label="价格颜色" />
                <ColorField data={data} onChange={onChange} field="marketPriceColor" label="划线价颜色" />
                <TextField data={buy} onChange={setBuy} field="text" label="按钮文字" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
