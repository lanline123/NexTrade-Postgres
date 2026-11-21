/**
* 商品 设置（对齐 view/admin editor/modules/mobile/product/Settings.vue）
*
* 结构与原版一致：选择模板（7 种）→ 商品来源（CommonProductEdit →
* ProductsSourceField）→ CommonActivityFrameEdit 整套商品卡设置（商品卡样式 /
* 瀑布流（仅 style==2）/ 商品卡边角 / 文本对齐 / 文字粗细 / 商品名称行数 /
* 商品信息边距 / 显示商品名称 / 显示商品价格 / 商品边距（max 50）/ 购买按钮
* 样式 0–8）→ 容器设置（背景颜色 / 圆角 / 内边距 / 左右 / 上 / 下边距，滑杆
* 上限均为 30）→ 指示器颜色（style==7，无分组标题）→ 模块标题（type=product）。
* 原版 product 无 frame 编辑区（CommonFrameEdit）与 imgPadding 控件——两字段
* 仅存在于默认值、由 Preview 消费（imgPadding 实际不消费），此处不提供入口。
* 「选择模板」原版为 ico-decorate 图标 radio，现走共享 TemplateIconRadioField
* 视觉选择器；
* 模板组提示文案此前无左内边距、与字段标签不对齐，现补 padding 0 16px 同左缘。
*/
import { SettingGroup, RadioRowField, BuyBtnStyleRowField, RangeField, ColorField, TitleSettingsField} from "../../../shared/settingWidgets";
import { TemplateIconRadioField} from "../../../shared/templateIcon";
import ProductsSourceField from "../../../shared/ProductsSourceField";

const STYLE_OPTIONS = [
{ value: 1, label: "一行一个", icon: "up2end"},
{ value: 2, label: "一行两个", icon: "small"},
{ value: 3, label: "一行三个", icon: "three"},
{ value: 4, label: "一大两小", icon: "hybrid"},
{ value: 5, label: "详细列表", icon: "list"},
{ value: 6, label: "横向滑动", icon: "swipe"},
{ value: 7, label: "轮播滑动", icon: "indicator-1"},
];

const SHOW_OPTIONS = [
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"}
];

export default function ProductSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
return (
<div className="h5-editor-settings-body">
<SettingGroup title="商品">
<div style={{ color: '#646a73', fontSize: 12, marginBottom: 8, padding: "0 16px", marginTop: 10, lineHeight: 1.6}}>
提示：仅“一行两个”模板类型支持<b style={{ color: '#333'}}>瀑布流</b>展示，为了展示效果，请尽量将瀑布流组件置于底部，一个页面不要超过一个
</div>
<TemplateIconRadioField data={data} onChange={onChange} field="style" label="选择模板" options={STYLE_OPTIONS} />
<ProductsSourceField data={data} onChange={onChange} />
</SettingGroup>
<SettingGroup>
<RadioRowField
data={data}
onChange={onChange}
field="goodsStyle"
label="商品卡样式"
options={[
{ value: 1, label: "标准"},
{ value: 2, label: "投影"},
{ value: 3, label: "描边"},
{ value: 4, label: "透明底"}
]}
/>
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
<RadioRowField
data={data}
onChange={onChange}
field="goodsRadioStyle"
label="商品卡边角"
options={[
{ value: 1, label: "直角"},
{ value: 2, label: "圆角"}
]}
/>
<RadioRowField
data={data}
onChange={onChange}
field="textAlign"
label="文本对齐"
options={[
{ value: 1, label: "左对齐"},
{ value: 2, label: "居中对齐"}
]}
/>
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
<RadioRowField data={data} onChange={onChange} field="showPrice" label="显示商品价格" options={SHOW_OPTIONS} />
<RangeField data={data} onChange={onChange} field="goodsPadding" label="商品边距" max={50} />
<BuyBtnStyleRowField
data={data}
onChange={onChange}
field="buyBtnStyle"
label="购买按钮样式"
/>
</SettingGroup>
<SettingGroup>
<ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue="#ffffff" />
<RangeField data={data} onChange={onChange} field="boxRadius" label="圆角" max={30} />
<RangeField data={data} onChange={onChange} field="innerPadding" label="内边距" max={30} />
<RangeField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={30} />
<RangeField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} />
<RangeField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} />
</SettingGroup>
{Number(data?.style) === 7? (
<SettingGroup>
<ColorField data={data} onChange={onChange} field="swiperPageColor" label="指示器颜色" />
</SettingGroup>
): null}
<TitleSettingsField data={data} onChange={onChange} titleType="product" />
</div>
);
}
