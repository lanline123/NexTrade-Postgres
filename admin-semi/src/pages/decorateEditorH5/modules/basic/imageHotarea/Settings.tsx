/**
* 热图绘制 设置（字段对齐 view/admin imageHotarea/Settings.vue + components/decorate ImageHotarea.vue）
*
* 结构与 PC 编辑器 pc-image-hotarea 同构（2026-11-20 用户指令：H5 向 PC 看齐，
* 废止此前「表单化」改口径）：
* - 「热图」走共享 PicListField（素材库选图、标题、条目链接，与 PC PicListEditor
* 实际渲染同口径：def 未设 picNoLink/picNoText，标题与链接均显示）；
* - 「热区绘制」走 HotareaPainter 画板（自 PcDecorateEditor 逐字移植至 shared/）：
* 在图上拖拽框选/移位/缩放热区，坐标按 500px 基准存储，右键菜单或 inspector 设链，
* 热区链接为完整 LinkType 对象（hotarea.link）。
* 数据契约与旧实现完全同构：picList[{picUrl,picThumb,picTitle,picLink?,hotarea[
* {left,top,width,height,link}]}]，零迁移。新加图片条目暂无 hotarea 键（画板按
* 空数组处理，绘制后写入），与缺省等价。
*
*/
import { SettingGroup, PicListField, TitleSettingsField, FrameSettingsField} from "../../../shared/settingWidgets";
import HotareaPainter from "../../../shared/HotareaPainter";
import { useI18n} from '../../../../../i18n';

export default function ImageHotareaSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const { t} = useI18n();
const picList: any[] = Array.isArray(data?.picList)? data.picList: [];

return (
<div className="h5-editor-settings-body">
<div style={{ fontSize: 12, color: "#999", padding: "0 16px", marginBottom: 4}}>{t('添加图片后，在下方「热区绘制」中直接在图上拖拽框选热区')}</div>
<PicListField data={data} onChange={onChange} field="picList" label={t('热图')} />
<SettingGroup title={t('热区绘制')}>
<HotareaPainter picList={picList} onChange={(next) => onChange({ picList: next})} />
</SettingGroup>
<TitleSettingsField data={data} onChange={onChange} />
<FrameSettingsField data={data} onChange={onChange} />
</div>
);
}
