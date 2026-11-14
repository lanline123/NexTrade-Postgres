/**
 * 分类推荐A1 设置
 */
import { SettingGroup, NumberField, ColorField, SelectField, PicListField } from "../../../shared/settingWidgets";

export default function CategoryRecA1Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const subTitle = data?.subTitle ?? {};
    const setSub = (p: Record<string, any>) => onChange({ subTitle: { ...subTitle, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="分类推荐">
                <PicListField data={data} onChange={onChange} field="picList" label="推荐图片" max={10} />
                <NumberField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={40} suffix="px" />
                <SelectField data={data} onChange={onChange} field="picFillType" label="图片填充" options={[{ value: "cover", label: "填充" }, { value: "contain", label: "留白" }]} />
                <ColorField data={data} onChange={onChange} field="subTitleColor" label="描述颜色" />
                <NumberField data={subTitle} onChange={setSub} field="titleFontSize" label="标题字号" min={10} max={24} suffix="px" />
                <ColorField data={subTitle} onChange={setSub} field="titleColor" label="标题颜色" />
            </SettingGroup>
        </div>
    );
}
