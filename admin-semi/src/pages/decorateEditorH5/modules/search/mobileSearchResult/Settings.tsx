/**
 * 搜索结果 设置（字段与渲染端 searchResultDefaults 一一对应；门店字段仅 searchResultType=2 时展示）
 */
import { SettingGroup, TextField, SwitchField, RadioRowField, NumberField, ColorField } from "../../../shared/settingWidgets";

export default function MobileSearchResultSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const isStore = Number(data?.searchResultType) === 2;
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="基础">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="searchResultType"
                    label="结果类型"
                    options={[
                        { value: 1, label: "商品搜索" },
                        { value: 2, label: "门店聚合" }
                    ]}
                />
                <TextField data={data} onChange={onChange} field="placeholder" label="搜索提示文字" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="layoutStyle"
                    label="布局样式"
                    options={[
                        { value: 1, label: "国内版" },
                        { value: 2, label: "海外版" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="listStyle"
                    label="列表样式"
                    options={[
                        { value: 1, label: "双列" },
                        { value: 2, label: "单列" }
                    ]}
                />
            </SettingGroup>
            <SettingGroup title="颜色">
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="页面背景色" />
                <ColorField data={data} onChange={onChange} field="searchItemBackgroundColor" label="搜索区背景色" />
                <ColorField data={data} onChange={onChange} field="itemBackgroundColor" label="商品项背景色" />
                <ColorField data={data} onChange={onChange} field="themeColor" label="主题色" />
                <ColorField data={data} onChange={onChange} field="productTitleColor" label="商品标题色" />
                <ColorField data={data} onChange={onChange} field="priceColor" label="价格颜色" />
            </SettingGroup>
            <SettingGroup title="商品展示">
                <NumberField data={data} onChange={onChange} field="productTitleLine" label="标题行数" min={1} max={2} />
                <NumberField data={data} onChange={onChange} field="buyBtnStyle" label="购物车按钮样式(0-8,双列)" min={0} max={8} />
                <NumberField data={data} onChange={onChange} field="productImageRadius" label="商品卡圆角" max={30} suffix="px" />
                <SwitchField data={data} onChange={onChange} field="showShopTag" label="显示店铺标签(单列)" />
                <SwitchField data={data} onChange={onChange} field="showSales" label="显示销量(单列)" />
                <SwitchField data={data} onChange={onChange} field="showCartButton" label="显示购物车按钮(单列)" />
                <SwitchField data={data} onChange={onChange} field="showCoupon" label="显示优惠券(双列)" />
            </SettingGroup>
            <SettingGroup title="间距">
                <NumberField data={data} onChange={onChange} field="searchItemPaddingLeftRight" label="搜索区左右间距" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="margin" label="模块外边距" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="paddingTopBottom" label="上下内间距" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="paddingLeftRight" label="左右内间距" max={40} suffix="px" />
            </SettingGroup>
            {isStore ? (
                <SettingGroup title="门店展示">
                    <SwitchField data={data} onChange={onChange} field="storeShowRating" label="显示评分" />
                    <SwitchField data={data} onChange={onChange} field="storeShowMinOrder" label="显示起送价" />
                    <SwitchField data={data} onChange={onChange} field="storeShowDeliveryFee" label="显示配送费" />
                    <SwitchField data={data} onChange={onChange} field="storeShowDistance" label="显示距离" />
                    <SwitchField data={data} onChange={onChange} field="storeShowFollow" label="显示关注按钮" />
                    <SwitchField data={data} onChange={onChange} field="storeShowProductName" label="显示门店商品名" />
                    <NumberField data={data} onChange={onChange} field="storeProductNameRow" label="门店商品名行数" min={1} max={2} />
                    <SwitchField data={data} onChange={onChange} field="storeShowProductPrice" label="显示门店商品价" />
                    <NumberField data={data} onChange={onChange} field="storeProductGap" label="门店商品间距" max={30} suffix="px" />
                    <NumberField data={data} onChange={onChange} field="storeGap" label="门店卡片间距" max={30} suffix="px" />
                    <NumberField data={data} onChange={onChange} field="storePaddingTopBottom" label="卡片上下内间距" max={40} suffix="px" />
                    <NumberField data={data} onChange={onChange} field="storePaddingLeftRight" label="卡片左右内间距" max={40} suffix="px" />
                    <NumberField data={data} onChange={onChange} field="storeCardRadius" label="卡片圆角" max={30} suffix="px" />
                    <NumberField data={data} onChange={onChange} field="storeLogoRadius" label="Logo圆角" max={50} suffix="%" />
                    <ColorField data={data} onChange={onChange} field="storeShopTitleColor" label="门店名称色" />
                    <NumberField data={data} onChange={onChange} field="storeShopTitleFontSize" label="门店名称字号" min={10} max={24} suffix="px" />
                    <NumberField data={data} onChange={onChange} field="storeShopTitleFontBold" label="门店名称粗细" min={400} max={700} step={300} />
                </SettingGroup>
            ) : null}
        </div>
    );
}
