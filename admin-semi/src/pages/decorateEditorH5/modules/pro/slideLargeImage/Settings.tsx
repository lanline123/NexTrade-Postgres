/**
 * 大图滑动 设置
 */
import { SettingGroup, NumberField, SelectField, PicListField, TextField } from "../../../shared/settingWidgets";

export default function SlideLargeImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const mt = data?.moduleTitle ?? {}; const setMt = (p: Record<string, any>) => onChange({ moduleTitle: { ...mt, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="大图列表">
                <PicListField data={data} onChange={onChange} field="picList" label="大图图片" max={10} />
            </SettingGroup>
            <SettingGroup title="模块标题">
                <TextField data={mt} onChange={setMt} field="title" label="标题文字" />
                <SelectField data={mt} onChange={setMt} field="titlePosition" label="标题位置" options={[{ value: "left", label: "居左" }, { value: "center", label: "居中" }, { value: "right", label: "居右" }]} />
            </SettingGroup>
            <SettingGroup title="样式设置">
                <NumberField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="picMargin" label="图片左右间距" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="picMarginTop" label="上间距" max={40} suffix="px" />
                <NumberField data={data} onChange={onChange} field="slidingSpeed" label="滑动速度" min={1000} max={10000} step={500} suffix="ms" />
                <SelectField data={data} onChange={onChange} field="slidingEffect" label="滑动效果" options={[{ value: "slide", label: "滑动" }, { value: "fade", label: "淡入" }]} />
            </SettingGroup>
        </div>
    );
}
