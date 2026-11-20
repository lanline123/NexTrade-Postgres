/**
* 个性商品 设置（对齐 view/admin mobilePro/customProduct/Settings.vue）
*
* 页签：商品设置 / 扩展设置。
* 商品选择与原版 Settings.vue 同口径：ProductInfoSelect v-model:productList 写回
* pruductList（字段名沿原版拼写），isMultiple + maxSelection=4（即分组标题
* 「最多选择4个」，上限由选品弹窗承接，与原版 maxSelection 透传 SelectProduct
* 弹窗的口径一致）；已选商品可删除、可拖拽排序（共享 ProductInfoSelect 承接，
* 见 shared/ProductInfoSelect 头注）。
* 扩展设置：颜色设置（moduleColor，划线价色仅 crossedOutPrice=1 时出现）
* + ModuleStyle/ContentStyle type="customProduct"。
* 「海报位置」原版为 iconfont-admin 图标钮（icon-alignLeft/icon-alignRight），
* 2026-11-20 起走共享视觉选择器。
*/
import { SettingGroup, RadioRowField, RangeField, ColorField, SinglePicField, DecDivider} from "../../../shared/settingWidgets";
import { TemplateIconRadioRowField} from "../../../shared/templateIcon";
import { SettingsTabs, ExtensionStyleSettings, AnimationSettingsField, PicFillTypeField} from "../../../shared/styleEdits";
import ProductInfoSelect from "../../../shared/ProductInfoSelect";

export default function CustomProductSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const moduleColor = data?.moduleColor?? {};
const setColor = (patch: Record<string, any>) => onChange({ moduleColor: {...moduleColor,...patch}});

return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["商品设置", "扩展设置"]}>
<>
<SettingGroup title="请点击添加选择商品：最多选择4个">
<ProductInfoSelect
value={Array.isArray(data?.pruductList)? data.pruductList: []}
onChange={(next) => onChange({ pruductList: next})}
isMultiple
maxSelection={4}
/>
<div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6, marginTop: 8}}>
提示：您可以通过拖拽进行商品排序
</div>
</SettingGroup>
<SettingGroup title="海报设置：宽280, 高600">
<SinglePicField data={data} onChange={onChange} field="picUrl" label="" />
</SettingGroup>
<TemplateIconRadioRowField
data={data}
onChange={onChange}
field="picLocation"
label="海报位置"
options={[
{ value: "left", label: "左侧", icon: "align-left"},
{ value: "right", label: "右侧", icon: "align-right"},
]}
/>
<SettingGroup title="参数设置">
<AnimationSettingsField data={data} onChange={onChange} type="categoryA1" />
<DecDivider />
<PicFillTypeField data={data} onChange={onChange} field="picFillType" />
<RangeField data={data} onChange={onChange} field="productRadius" label="商品圆角半径" max={20} />
<RadioRowField
data={data}
onChange={onChange}
field="picBottomRadius"
label="图片底部圆角"
options={[
{ value: 1, label: "直角"},
{ value: 2, label: "圆角"},
]}
/>
<RadioRowField
data={data}
onChange={onChange}
field="crossedOutPrice"
label="商品划线价"
options={[
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"},
]}
/>
</SettingGroup>
</>
<>
<SettingGroup title="颜色设置">
<ColorField data={moduleColor} onChange={setColor} field="productBackgroundColor" label="商品背景颜色" resetValue="" />
<ColorField data={moduleColor} onChange={setColor} field="productNameColor" label="商品标题颜色" resetValue="" />
<ColorField data={moduleColor} onChange={setColor} field="productPriceColor" label="商品价格颜色" resetValue="" />
{Number(data?.crossedOutPrice) === 1 && (
<ColorField data={moduleColor} onChange={setColor} field="crossedOutPriceColor" label="商品划线价颜色" resetValue="" />
)}
</SettingGroup>
<ExtensionStyleSettings data={data} onChange={onChange} type="customProduct" />
</>
</SettingsTabs>
</div>
);
}
