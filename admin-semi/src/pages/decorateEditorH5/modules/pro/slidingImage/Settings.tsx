/**
 * 图片滑动 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, SelectField, PicListField } from "../../../shared/settingWidgets";

export default function SlidingImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="图片列表">
                <PicListField data={data} onChange={onChange} field="picList" label="滑动图片" max={20} />
            </SettingGroup>
            <SettingGroup title="样式设置">
                <SwitchField data={data} onChange={onChange} field="autoPlay" label="自动播放" />
                <NumberField data={data} onChange={onChange} field="interval" label="播放间隔" min={1000} max={10000} step={500} suffix="ms" />
                <NumberField data={data} onChange={onChange} field="picWidth" label="图片宽度" min={30} max={100} suffix="%" />
                <NumberField data={data} onChange={onChange} field="picMargin" label="图片间距" max={30} suffix="px" />
                <NumberField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={40} suffix="px" />
                <SelectField data={data} onChange={onChange} field="picFillType" label="图片填充" options={[{ value: "cover", label: "填充" }, { value: "contain", label: "留白" }]} />
                <SwitchField data={data} onChange={onChange} field="showDesc" label="显示描述" />
                <ColorField data={data} onChange={onChange} field="descColor" label="描述颜色" />
            </SettingGroup>
        </div>
    );
}
