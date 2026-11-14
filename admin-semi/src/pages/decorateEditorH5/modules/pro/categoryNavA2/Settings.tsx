/**
 * 分类导航A2 设置
 */
import { SettingGroup, RadioRowField, SelectField, NumberField, ColorField, SwitchField, PicListField } from "../../../shared/settingWidgets";

export default function categoryNavA2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="分类导航">
                <PicListField data={data} onChange={onChange} field="categoryList" label="分类图片" max={30} />
                <RadioRowField data={data} onChange={onChange} field="listStyleType" label="列表样式" options={[{ value: "list", label: "固定" }, { value: "slide", label: "横向滑动" }]} />
                <NumberField data={data} onChange={onChange} field="rowNum" label="每行数量" min={2} max={6} />
                <NumberField data={data} onChange={onChange} field="picPercent" label="图片百分比" min={50} max={100} suffix="%" />
                <SelectField data={data} onChange={onChange} field="picFillType" label="图片填充" options={[{ value: "cover", label: "填充" }, { value: "contain", label: "留白" }]} />
                <NumberField data={data} onChange={onChange} field="radius" label="圆角" max={200} suffix="px" />
                <NumberField data={data} onChange={onChange} field="categoryPadding" label="分类间距" max={30} suffix="px" />
            </SettingGroup>
            <SettingGroup title="指示器">
                <SwitchField data={data} onChange={onChange} field="indicatorBar" label="显示指示器" />
                <ColorField data={data} onChange={onChange} field="barColor" label="指示器颜色" />
                <ColorField data={data} onChange={onChange} field="barColorOn" label="指示器选中颜色" />
                <NumberField data={data} onChange={onChange} field="barWidth" label="指示器宽度" max={60} suffix="px" />
                <NumberField data={data} onChange={onChange} field="barWidthOn" label="选中宽度" max={60} suffix="px" />
                <NumberField data={data} onChange={onChange} field="barHeight" label="指示器高度" max={10} suffix="px" />
            </SettingGroup>
        </div>
    );
}
