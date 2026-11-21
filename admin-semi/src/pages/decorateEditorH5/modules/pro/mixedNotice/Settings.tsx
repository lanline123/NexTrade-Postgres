/**
* 公告混排 设置（对齐 view/admin mobilePro/mixedNotice/Settings.vue）
*
* 页签：内容设置 / 扩展设置。
* 内容页仅 ComboGroupNoticeEdit（分组折叠卡 + 组内 PicList，含图片/标题/描述/链接，
* 提示「图片: 宽20, 高20」）；原版 picScale/picFillType 已注释不生效，不提供编辑。
* 扩展页：颜色设置（大标题颜色/小标题颜色，键 moduleColor）+ ModuleStyleEdit +
* ContentStyleEdit（type="mixedProductDisplay"）。
*/
import { SettingGroup, ColorField, PicListEditorField} from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings} from "../../../shared/styleEdits";
import GroupCardHeader from "../../../../../components/GroupCardHeader";

import { useI18n} from '../../../../../i18n';
function NoticeGroupsEditor({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const { t} = useI18n();
const groups: Array<Record<string, any>> = Array.isArray(data?.noticeGroups)? data.noticeGroups: [];
const setGroup = (index: number, patch: Record<string, any>) =>
onChange({ noticeGroups: groups.map((g, i) => (i === index? {...g,...patch}: g))});
const moveGroup = (index: number, delta: number) => {
const target = index + delta;
if (target < 0 || target >= groups.length) return;
const next = [...groups];
const [g] = next.splice(index, 1);
next.splice(target, 0, g);
onChange({ noticeGroups: next});
};
return (
<div style={{ padding: "0 10px 15px"}}>
{groups.map((group, index) => (
<div
key={index}
style={{
width: "100%",
position: "relative",
marginTop: 12,
borderRadius: 2,
backgroundColor: "#fff",
boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)"
}}
>
<GroupCardHeader
title={`第${index + 1}组`}
isFirst={index === 0}
isLast={index === groups.length - 1}
onMove={(delta) => moveGroup(index, delta)}
/>
<div className="dec-edit-group">
<div style={{ width: "100%"}}>
<div style={{ color: "var(--semi-color-primary)", fontSize: 12, marginBottom: 4}}>{t('图片: 宽20, 高20')}</div>
<PicListEditorField
data={group}
onChange={(patch) => setGroup(index, patch)}
field="notices"
label=""
showDesc
addText="添加公告"
/>
</div>
</div>
</div>
))}
</div>
);
}

export default function MixedNoticeSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const moduleColor = data?.moduleColor?? {};
const setColor = (patch: Record<string, any>) => onChange({ moduleColor: {...moduleColor,...patch}});
return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["内容设置", "扩展设置"]}>
<NoticeGroupsEditor data={data} onChange={onChange} />
<>
<SettingGroup title="颜色设置">
<ColorField data={moduleColor} onChange={setColor} field="titleColor" label="大标题颜色" resetValue="" />
<ColorField data={moduleColor} onChange={setColor} field="sellingPointColor" label="小标题颜色" resetValue="" />
</SettingGroup>
<ExtensionStyleSettings data={data} onChange={onChange} type="mixedProductDisplay" />
</>
</SettingsTabs>
</div>
);
}
