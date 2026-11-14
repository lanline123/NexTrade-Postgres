/**
 * 店铺导航 设置（渲染端暂无入口，字段先按契约做全，待渲染端补入口后直接生效）
 */
import { SettingGroup, NumberField, SelectField, PicListField } from "../../../shared/settingWidgets";

export default function StoreNavSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="店铺导航">
                <div style={{ color: "#646a73", fontSize: 12, marginBottom: 8 }}>渲染端暂无入口，本模块不进工具栏，仅保证存量数据可编辑/预览。</div>
                <PicListField data={data} onChange={onChange} field="picList" label="导航图片" max={10} />
                <NumberField data={data} onChange={onChange} field="rowNum" label="每行数量" min={2} max={6} />
                <NumberField data={data} onChange={onChange} field="picPercent" label="图片百分比" min={50} max={100} suffix="%" />
                <SelectField data={data} onChange={onChange} field="picFillType" label="图片填充" options={[{ value: "cover", label: "填充" }, { value: "contain", label: "留白" }]} />
                <NumberField data={data} onChange={onChange} field="radius" label="圆角" max={200} suffix="px" />
                <NumberField data={data} onChange={onChange} field="categoryPadding" label="条目间距" max={30} suffix="px" />
            </SettingGroup>
        </div>
    );
}
