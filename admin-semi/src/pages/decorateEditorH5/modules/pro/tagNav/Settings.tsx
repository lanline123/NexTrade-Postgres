/**
* 标签导航 设置（对齐 view/admin mobilePro/tagNav/Settings.vue）
*
* 页签：分组设置（基本设置 GroupListEdit + 参数设置）/ 扩展设置
* （颜色设置：categoryColors 渐变色对 + 分组标题色；ModuleStyle type="categoryRec"）。
* 分组链接已恢复原版选链方式：PicLinkField（原版 GroupListEdit 以 SelectLink 绑定
* element.link 整体，选中即回写完整 LinkType；默认占位 { link:{link,title}, linkTitle} 保留）。
* 「文本样式」原版为 iconfont-admin 字形钮（icon-bianji-ziti 常规/加粗），
* 2026-11-20 起走共享视觉选择器。
*/
import { SettingGroup, SettingRow, RangeField, ColorField, TextField, PicLinkField} from "../../../shared/settingWidgets";
import { TemplateIconRadioRowField} from "../../../shared/templateIcon";
import { SettingsTabs, ModuleStyleSettingsField} from "../../../shared/styleEdits";
import GroupListEditor from "../../../shared/GroupListEditor";
import AlphaColorInput from "../../../../../components/AlphaColorInput";

const EMPTY_LINK = { link: { link: "", title: ""}, linkTitle: ""};

function TagNavGroupEditor({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
return (
<GroupListEditor
data={data}
onChange={onChange}
field="groupList"
addText="添加分组推荐"
createItem={() => ({ title: "", link: JSON.parse(JSON.stringify(EMPTY_LINK))})}
canRemove={(list) => list.length > 1}
renderItem={(item, _index, itemProps) => (
<>
<TextField {...itemProps} field="title" label="标题" placeholder="请输入标题内容" />
<SettingRow label="链接">
<PicLinkField
value={typeof item.link === "string"? { link: item.link, title: ""}: item.link?? null}
onChange={(v) => itemProps.onChange({ link: v})}
/>
</SettingRow>
</>
)}
/>
);
}

export default function TagNavSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const categoryColors: Array<{ colorA?: string; colorB?: string}> = Array.isArray(data?.categoryColors)? data.categoryColors: [];
const setColorPair = (index: number, patch: Record<string, any>) =>
onChange({ categoryColors: categoryColors.map((c, i) => (i === index? {...c,...patch}: c))});

return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["分组设置", "扩展设置"]}>
<>
<SettingGroup title="基本设置">
<TagNavGroupEditor data={data} onChange={onChange} />
</SettingGroup>
<SettingGroup title="参数设置">
<TemplateIconRadioRowField
data={data}
onChange={onChange}
field="textWeight"
label="文本样式"
options={[
{ value: 400, label: "常规体", aSize: 14},
{ value: 700, label: "加粗体", aSize: 14, aBold: true},
]}
/>
<RangeField data={data} onChange={onChange} field="groupGap" label="分组间距" max={30} />
<RangeField data={data} onChange={onChange} field="groupPadding" label="分组左右边距" max={30} />
</SettingGroup>
</>
<>
<SettingGroup title="颜色设置">
{categoryColors.map((c, index) => (
<SettingRow key={index} label={`第${index + 1}个分类：`}>
{/* 原版.dec-color-gradient：两色 + 「~」分隔（margin 0 8px、#969799），无重置链 */}
<div style={{ display: "flex", alignItems: "center"}}>
<AlphaColorInput value={c.colorA?? ""} onChange={(v) => setColorPair(index, { colorA: v})} />
<span style={{ margin: "0 8px", color: "#969799"}}>~</span>
<AlphaColorInput value={c.colorB?? ""} onChange={(v) => setColorPair(index, { colorB: v})} />
</div>
</SettingRow>
))}
<ColorField data={data} onChange={onChange} field="titleColor" label="分组标题色" resetValue="#ffffff" />
</SettingGroup>
<ModuleStyleSettingsField data={data} onChange={onChange} type="categoryRec" />
</>
</SettingsTabs>
</div>
);
}
