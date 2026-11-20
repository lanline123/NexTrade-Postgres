/**
* 图片广告1 设置（字段对齐 view/admin imageAd/Settings.vue）
*
* picType 说明文案逐字取自原版 Settings.vue「添加图片」组的 pic-change-desc（仅 picType 1~4 有文案，5~7 无）。
* 「选择模板」与「指示器样式」原版为 ico-decorate 图标 radio，现走共享
* TemplateIconRadioField / TemplateIconRadioRowField 视觉选择器。
* 提示文案原版在「添加图片」组内（标题之下、PicList 之上，.dec-edit-group-desc
* 随组 padding 左对齐）；此前我方把它写成设置体直接子 div（无 padding、左缘 0），
* 与字段标签左缘（16px）不对齐（用户 2026-11-20 点名）——现以同左缘（padding 0 16px）
* 的 desc 块紧贴 PicListField 之上渲染，左缘与其它字段一致。
*/
import {
SettingGroup,
RadioRowField,
NumberField,
ColorField,
PicListEditorField,
TitleSettingsField,
FrameSettingsField
} from "../../../shared/settingWidgets";
import { TemplateIconRadioField, TemplateIconRadioRowField} from "../../../shared/templateIcon";

/** 原版 picType 说明文案（逐字） */
const PIC_TYPE_DESC: Record<number, string> = {
1: "提示：您可以通过拖拽进行图片排序，单图占屏幕宽为100%，建议宽750px，高度自适应",
2: "提示：您可以通过拖拽进行图片排序，建议手机端尺寸750x350px，高度统一；PC端宽不小于1500px（PC端选择时轮播默认通宽）",
3: "提示：您可以通过拖拽进行图片排序，单图占屏幕宽为80%，建议宽度600px，高度统一",
4: "提示：您可以通过拖拽进行图片排序，单图占屏幕宽为40%，建议宽度300px，高度统一"
};

export default function ImageAdSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const picTypeDesc = PIC_TYPE_DESC[Number(data?.picType)];
return (
<div className="h5-editor-settings-body">
<SettingGroup title="模板样式">
<TemplateIconRadioField
data={data}
onChange={onChange}
field="picType"
label="选择模板"
options={[
{ value: 1, label: "一行一个", icon: "up2end"},
{ value: 2, label: "轮播海报", icon: "carousel"},
{ value: 3, label: "横滑大图", icon: "big-slide"},
{ value: 4, label: "横滑小图", icon: "small-slide"},
{ value: 5, label: "一行两个", icon: "cuberow"},
{ value: 6, label: "一行三个", icon: "cuberow2"},
{ value: 7, label: "一行四个", icon: "cuberow1"}
]}
/>
{Number(data?.picType) === 2 && (
<>
<RadioRowField
data={data}
onChange={onChange}
field="swiperPreView"
label="轮播行显示数量"
options={[
{ value: 1, label: "1个"},
{ value: 2, label: "2个"},
{ value: 3, label: "3个"}
]}
/>
<TemplateIconRadioRowField
data={data}
onChange={onChange}
field="picPageType"
label="指示器样式"
options={[
{ value: 1, label: "样式一", icon: "indicator-1"},
{ value: 2, label: "样式二", icon: "indicator-2"},
{ value: 3, label: "样式三", icon: "indicator-3"}
]}
/>
<ColorField data={data} onChange={onChange} field="swiperPageColor" label="指示器颜色" />
</>
)}
<NumberField data={data} onChange={onChange} field="imgPadding" label="图片边距" suffix="px" max={30} />
<RadioRowField
data={data}
onChange={onChange}
field="picRadioStyle"
label="图片边角"
options={[
{ value: 1, label: "直角"},
{ value: 2, label: "圆角"}
]}
/>
<RadioRowField
data={data}
onChange={onChange}
field="isFluxWidth"
label="是否通宽排版(仅PC端有效)"
options={[
{ value: 1, label: "是"},
{ value: 0, label: "否"}
]}
/>
</SettingGroup>
{picTypeDesc? (
<div style={{ padding: "0 16px", marginTop: 10}}>
<div className="pic-change-desc" style={{ color: "#999", fontSize: 12, lineHeight: "18px"}}>
{picTypeDesc}
</div>
</div>
): null}
<PicListEditorField data={data} onChange={onChange} field="picList" label="图片广告" />
<TitleSettingsField data={data} onChange={onChange} />
<FrameSettingsField data={data} onChange={onChange} />
</div>
);
}
