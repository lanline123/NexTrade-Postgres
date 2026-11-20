/**
* 商品A1 设置（对齐 view/admin mobilePro/productA1/Settings.vue）
*
* 页签：基础设置 / 扩展设置（CommonActivityFrameEdit section="container"）。
* CommonActivityFrameEdit（D22 未移植）按现行接法拆解：内容段 = ProductsSourceField
* + 商品卡字段本地组（type=product、use-goods-radius-slider、hide-text-align、
* hide-buy-btn-style 的门控逐项落实，插槽字段 nameAlign/priceAlign/showMarketPrice/
* itemGap 按原版位置插入）；容器段 = 模块顶层 backgroundColor/圆角/边距字段组。
* BuyBtnEdit 按 buyBtn/Edit.vue 字段集本地实现（buyBtnStyle 对象，历史数字值先回退默认对象）。
* 选择模板原版为图标 radio，现走共享 TemplateIconRadioField 视觉选择器
* 。
*/
import { Input, TextArea} from "@douyinfe/semi-ui";
import {
SettingGroup,
SettingRow,
RadioRowField,
RadioField,
RangeField,
ColorField,
TextField,
PicSelectField,
DecDivider
} from "../../../shared/settingWidgets";
import { TemplateIconRadioField} from "../../../shared/templateIcon";
import { SettingsTabs} from "../../../shared/styleEdits";
import { cloneDeep, defaultProductA1} from "../../../contract/sharedDefaults";
import ProductsSourceField from "../../../shared/ProductsSourceField";
import { useI18n} from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
data: Record<string, any>;
onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"}
];
const ALIGN_OPTIONS = [
{ value: 1, label: "左对齐"},
{ value: 2, label: "居中对齐"}
];

function BuyBtnSettings({ data, onChange}: Props) {
const btn = data?.buyBtnStyle && typeof data.buyBtnStyle === "object"? data.buyBtnStyle: cloneDeep(defaultProductA1.buyBtnStyle);
const sub: Props = { data: btn, onChange: (patch: Patch) => onChange({ buyBtnStyle: {...btn,...patch}})};
return (
<SettingGroup title="购买按钮">
<RadioRowField
{...sub}
field="showBtn"
label="购买按钮"
options={[
{ value: 0, label: "不显示"},
{ value: 1, label: "显示"}
]}
/>
{Number(btn.showBtn) === 1? (
<>
<SettingRow label="购买按钮图标样式" block>
<RadioField
{...sub}
field="btnType"
options={[
{ value: 1, label: "样式一"},
{ value: 2, label: "样式二"},
{ value: 3, label: "样式三"},
{ value: 4, label: "样式四"},
{ value: 5, label: "自定义图片"}
]}
/>
</SettingRow>
{Number(btn.btnType) === 5? <PicSelectField {...sub} field="iconImage" label="自定义图标图片" />: null}
<RangeField {...sub} field="iconSize" label="图标尺寸" min={12} max={40} />
<TextField {...sub} field="text" label="文字内容" placeholder="请输入按钮文字" />
<RadioRowField
{...sub}
field="wrap"
label="按钮换行"
options={[
{ value: 0, label: "不换行"},
{ value: 1, label: "换行"}
]}
/>
{Number(btn.wrap) === 1? (
<RadioRowField
{...sub}
field="wrapAlign"
label="换行后对齐"
options={[
{ value: 1, label: "居左"},
{ value: 2, label: "居中"},
{ value: 3, label: "居右"}
]}
/>
): null}
<RangeField {...sub} field="textSize" label="文字字号" min={10} max={24} />
<ColorField {...sub} field="textColor" label="文字颜色" resetValue="#ffffff" />
<ColorField {...sub} field="iconColor" label="图标颜色" resetValue="#ffffff" />
<RadioRowField
{...sub}
field="gradientType"
label="背景渐变方向"
options={[
{ value: "upDown", label: "上下"},
{ value: "leftRight", label: "左右"},
{ value: "diagonal", label: "斜向"},
{ value: "purity", label: "纯色"}
]}
/>
<ColorField {...sub} field="gradientColorA" label="背景渐变颜色A" resetValue="#333333" />
<ColorField {...sub} field="gradientColorB" label="背景渐变颜色B" resetValue="#333333" />
<RangeField {...sub} field="radius" label="按钮圆角" max={20} />
<RangeField {...sub} field="paddingTopBottom" label="按钮高度" max={10} />
<RangeField {...sub} field="paddingLeftRight" label="按钮宽度" max={10} />
</>
): null}
</SettingGroup>
);
}

export default function ProductA1Settings({ data, onChange}: Props) {
const { t} = useI18n();
const products = data?.products?? {};
const productCount =
Number(products.productSelectType) === 1? (Array.isArray(products.productIds)? products.productIds.length: 0): Number(products.productNumber) || 0;
const isDynamic = Number(products.productSelectType) === 2 || Number(products.productSelectType) === 3;

const briefStyle = data?.briefStyle?? { showTitle: 1, titleColor: "#FF6600", titleFontSize: 11, titleFontBold: 400};
const briefProps: Props = { data: briefStyle, onChange: (patch: Patch) => onChange({ briefStyle: {...briefStyle,...patch}})};

const tagList: string[] = Array.isArray(data?.tagList)? data.tagList: [];
const briefList: string[] = Array.isArray(data?.briefList)? data.briefList: [];
const setListItem = (field: "tagList" | "briefList", list: string[], index: number, v: string) => {
const next = [...list];
while (next.length <= index) next.push("");
next[index] = v;
onChange({ [field]: next});
};

return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["基础设置", "扩展设置"]}>
<>
<TemplateIconRadioField
data={data}
onChange={onChange}
field="style"
label="选择模板"
options={[
{ value: 1, label: "一行一个", icon: "up2end"},
{ value: 2, label: "一行两个", icon: "small"},
{ value: 3, label: "一行三个", icon: "three"},
{ value: 4, label: "详细列表", icon: "list"}
]}
/>
<DecDivider />
<SettingGroup title="商品设置">
<ProductsSourceField data={data} onChange={onChange} />
<SettingRow label="商品卡样式" block>
<RadioField
data={data}
onChange={onChange}
field="goodsStyle"
options={[
{ value: 1, label: "标准"},
{ value: 2, label: "投影"},
{ value: 3, label: "描边"},
{ value: 4, label: "透明底"}
]}
/>
</SettingRow>
{Number(data?.style) === 2? (
<RadioRowField
data={data}
onChange={onChange}
field="waterfall"
label="瀑布流"
options={[
{ value: 1, label: "启用"},
{ value: 0, label: "否"}
]}
/>
): null}
<RangeField data={data} onChange={onChange} field="goodsRadius" label="商品卡圆角" max={30} />
<RadioRowField
data={data}
onChange={onChange}
field="textWeight"
label="文字粗细"
options={[
{ value: 1, label: "常规体"},
{ value: 2, label: "加粗体"}
]}
/>
<RadioRowField
data={data}
onChange={onChange}
field="goodsNameRow"
label="商品名称行数"
options={[
{ value: 1, label: "1行"},
{ value: 2, label: "2行"}
]}
/>
<RadioRowField
data={data}
onChange={onChange}
field="goodsNamePadding"
label="商品信息边距"
options={[
{ value: 1, label: "有边距"},
{ value: 0, label: "无边距"}
]}
/>
<RadioRowField data={data} onChange={onChange} field="showName" label="显示商品名称" options={SHOW_OPTIONS} />
<RadioRowField data={data} onChange={onChange} field="nameAlign" label="商品名称对齐" options={ALIGN_OPTIONS} />
<RadioRowField data={data} onChange={onChange} field="showPrice" label="显示商品价格" options={SHOW_OPTIONS} />
<RadioRowField data={data} onChange={onChange} field="priceAlign" label="价格对齐" options={ALIGN_OPTIONS} />
<RadioRowField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" options={SHOW_OPTIONS} />
<RangeField data={data} onChange={onChange} field="goodsPadding" label="商品边距" max={50} />
<RangeField data={data} onChange={onChange} field="itemGap" label="商品卡片间距" max={20} />
</SettingGroup>
<BuyBtnSettings data={data} onChange={onChange} />
<SettingGroup title="商品标签与卖点">
<RadioRowField data={data} onChange={onChange} field="showProductLabel" label="显示商品标签" options={SHOW_OPTIONS} />
{Number(data?.showProductLabel) === 1? (
<>
<ColorField data={data} onChange={onChange} field="labelBackgroundColor" label="标签背景色" resetValue="" />
<ColorField data={data} onChange={onChange} field="labelTextColor" label="标签文字色" resetValue="" />
<RangeField data={data} onChange={onChange} field="labelRadius" label="标签圆角值" max={20} />
<SettingRow label="圆角应用方式" block>
<RadioField
data={data}
onChange={onChange}
field="labelRadiusStyle"
options={[
{ value: 0, label: "全圆角"},
{ value: 1, label: "左边圆角"},
{ value: 2, label: "右边圆角"},
{ value: 3, label: "对角 ↖↘"},
{ value: 4, label: "对角 ↗↙"}
]}
/>
</SettingRow>
<RangeField data={data} onChange={onChange} field="labelFontSize" label="标签字号" min={9} max={16} />
<SettingRow label="标签位置" block>
<RadioField
data={data}
onChange={onChange}
field="labelPosition"
options={[
{ value: "top-left", label: "左上角"},
{ value: "top-right", label: "右上角"},
{ value: "bottom-left", label: "左下角"},
{ value: "bottom-right", label: "右下角"}
]}
/>
</SettingRow>
<RangeField data={data} onChange={onChange} field="labelOffsetX" label="水平边距" min={-10} max={30} />
<RangeField data={data} onChange={onChange} field="labelOffsetY" label="垂直边距" min={-10} max={30} />
</>
): null}
<RadioRowField data={data} onChange={onChange} field="showBrief" label="显示卖点描述" options={SHOW_OPTIONS} />
{Number(data?.showBrief) === 1? (
<>
<RangeField {...briefProps} field="titleFontSize" label="卖点字号" min={9} max={16} />
<ColorField {...briefProps} field="titleColor" label="卖点文字色" resetValue="" />
<RadioRowField
{...briefProps}
field="titleFontBold"
label="卖点粗细"
options={[
{ value: 400, label: "常规体"},
{ value: 700, label: "加粗体"}
]}
/>
<ColorField data={data} onChange={onChange} field="briefBackgroundColor" label="卖点徽章背景色" resetValue="" />
</>
): null}
</SettingGroup>
<DecDivider />
<SettingGroup title="每商品标签与卖点配置">
{productCount === 0? (
<div style={{ color: "#999", fontSize: 12, padding: "0 12px 10px", lineHeight: 1.6}}>
请先在上方「选择商品」中添加商品后再来配置标签与卖点
</div>
): (
<div style={{ padding: "0 10px 15px"}}>
{Array.from({ length: productCount}).map((_, index) => (
<div key={index} style={{ marginTop: 12, backgroundColor: "#fff", borderRadius: 4, boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.15)", paddingBottom: 8}}>
<div style={{ padding: "10px 12px", fontSize: 13, fontWeight: 600, borderBottom: "1px solid #f0f0f0", display: "flex", alignItems: "center", gap: 8}}>
第 {index + 1} 个商品
{isDynamic? (
<span style={{ fontSize: 11, fontWeight: 400, color: "#999", padding: "1px 6px", backgroundColor: "#f5f7fa", borderRadius: 2}}>动态拉取</span>
): null}
</div>
<SettingRow label="标签">
<Input value={tagList[index]?? ""} placeholder={t("如：新品 / 折扣")} maxLength={10} onChange={(v: string) => setListItem("tagList", tagList, index, v)} />
</SettingRow>
<SettingRow label="卖点" block>
<TextArea value={briefList[index]?? ""} placeholder={t("如：猜你喜欢")} autosize={{ minRows: 2, maxRows: 5}} onChange={(v: string) => setListItem("briefList", briefList, index, v)} />
</SettingRow>
</div>
))}
</div>
)}
</SettingGroup>
</>
<>
<SettingGroup title="容器设置">
<ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue="" />
<RangeField data={data} onChange={onChange} field="boxRadius" label="圆角" max={30} />
<RangeField data={data} onChange={onChange} field="innerPadding" label="内边距" max={30} />
<RangeField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={30} />
<RangeField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} />
<RangeField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} />
</SettingGroup>
</>
</SettingsTabs>
</div>
);
}
