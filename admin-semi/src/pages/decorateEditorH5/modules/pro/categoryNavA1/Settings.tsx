/**
* 类目导航A1 设置（对齐 view/admin mobilePro/categoryNavA1/Settings.vue）
*
* 页签：导航内容 / 扩展设置。
* 内容页：内容设置（PicList）+ 参数设置（列表样式/每屏显示数量/缓动效果/图片背景色/
* 图片填充/图片百分比/分类直角-圆角/分类间距/分类标题/分割线/图片形状）；
* 扩展页：ModuleStyleEdit + ContentStyleEdit（type="categoryA1"）。
* 注：原版「图片背景色」重置回 defaultFrame.backgroundColor（#ffffff），与原版 @click 口径一致。
* 「列表样式」原版为 iconfont-admin 图标钮，2026-11-20 起走共享视觉选择器
* 。
*/
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListEditorField, DecDivider, SegmentedRadio, SettingRow} from "../../../shared/settingWidgets";
import { TemplateIconRadioRowField} from "../../../shared/templateIcon";
import { SettingsTabs, ExtensionStyleSettings, AnimationSettingsField, SubTitleSettingsField, PicFillTypeField} from "../../../shared/styleEdits";
import { defaultFrame} from "../../../contract/sharedDefaults";

export default function CategoryNavA1Settings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["导航内容", "扩展设置"]}>
<>
<SettingGroup title="内容设置">
<PicListEditorField data={data} onChange={onChange} field="categoryList" label="" max={30} addText="添加一组" />
</SettingGroup>
<SettingGroup title="参数设置">
<TemplateIconRadioRowField
data={data}
onChange={onChange}
field="listStyleType"
label="列表样式"
options={[
{ value: "list", label: "固定", icon: "mode-fixed"},
{ value: "slide", label: "单列滑动", icon: "mode-slide"}
]}
/>
<SettingRow label="每屏显示数量" block>
<SegmentedRadio
value={Number(data?.rowNum?? 4)}
options={[2, 3, 4, 5, 6].map((n) => ({ value: n, label: `${n}个`}))}
onChange={(v) => onChange({ rowNum: v})}
/>
</SettingRow>
<AnimationSettingsField data={data} onChange={onChange} type="category_A2" />
<DecDivider />
<ColorField data={data} onChange={onChange} field="imageBackgroundColor" label="图片背景色" resetValue={defaultFrame.backgroundColor} />
<PicFillTypeField data={data} onChange={onChange} field="picFillType" />
<RangeField data={data} onChange={onChange} field="picPercent" label="图片百分比" max={100} />
<RangeField data={data} onChange={onChange} field="radius" label="分类直角-圆角" max={200} />
<RangeField data={data} onChange={onChange} field="categoryPadding" label="分类间距" max={30} />
<SubTitleSettingsField data={data} onChange={onChange} title="分类标题" />
<RadioRowField
data={data}
onChange={onChange}
field="showSplitLine"
label="分割线"
options={[
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"}
]}
/>
<RadioRowField
data={data}
onChange={onChange}
field="radiusStyle"
label="图片形状"
options={[
{ value: 1, label: "圆形"},
{ value: 2, label: "方形"}
]}
/>
</SettingGroup>
</>
<ExtensionStyleSettings data={data} onChange={onChange} type="categoryA1" />
</SettingsTabs>
</div>
);
}
