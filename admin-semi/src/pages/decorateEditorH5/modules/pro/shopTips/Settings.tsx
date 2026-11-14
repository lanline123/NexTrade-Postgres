/**
 * 店铺提示 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, PicListField } from "../../../shared/settingWidgets";

export default function ShopTipsSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="提示条目">
                <PicListField data={data} onChange={onChange} field="picList" label="提示图标与文字" max={6} />
                <div style={{ color: "#646a73", fontSize: 12 }}>条目标题在图片条目的 title 字段维护（与渲染端 picList[].title 一致）。</div>
            </SettingGroup>
            <SettingGroup title="样式设置">
                <NumberField data={data} onChange={onChange} field="rowNum" label="每行数量" min={1} max={5} />
                <NumberField data={data} onChange={onChange} field="itemGap" label="条目间距" max={30} suffix="px" />
                <ColorField data={data} onChange={onChange} field="titleColor" label="文字颜色" />
                <ColorField data={data} onChange={onChange} field="itemBackgroundColor" label="背景颜色" />
                <SwitchField data={data} onChange={onChange} field="showDivider" label="显示分隔线" />
            </SettingGroup>
        </div>
    );
}
