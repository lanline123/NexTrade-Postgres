/**
 * 类目导航 设置（对齐 view/admin mobilePro/categoryNav/Settings.vue）
 *
 * 页签：导航内容 / 扩展设置。
 * 内容页：内容设置（PicList）+ 参数设置（列表样式/每屏显示数量/缓动效果/
 * 图片填充/图片百分比/分类直角-圆角/分类间距/分类标题）；
 * 扩展页：ModuleStyleEdit + ContentStyleEdit（type="categoryA1"）。
 */
import { SettingGroup, RadioRowField, RangeField, PicListField, DecDivider, SegmentedRadio, SettingRow } from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings, AnimationSettingsField, SubTitleSettingsField, PicFillTypeField } from "../../../shared/styleEdits";

export default function CategoryNavSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs
                tabs={["导航内容", "扩展设置"]}
            >
                <>
                    <SettingGroup title="内容设置">
                        <PicListField data={data} onChange={onChange} field="categoryList" label="" max={30} addText="添加一组" />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="listStyleType"
                            label="列表样式"
                            options={[
                                { value: "list", label: "固定" },
                                { value: "slide", label: "横向滑动" },
                            ]}
                        />
                        <SettingRow label="每屏显示数量" block>
                            <SegmentedRadio
                                value={Number(data?.rowNum ?? 5)}
                                options={[2, 3, 4, 5, 6].map((n) => ({ value: n, label: `${n}个` }))}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        <AnimationSettingsField data={data} onChange={onChange} type="categoryA1" />
                        <DecDivider />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" />
                        <RangeField data={data} onChange={onChange} field="picPercent" label="图片百分比" max={100} />
                        <RangeField data={data} onChange={onChange} field="radius" label="分类直角-圆角" max={200} />
                        <RangeField data={data} onChange={onChange} field="categoryPadding" label="分类间距" max={30} />
                        <SubTitleSettingsField data={data} onChange={onChange} title="分类标题" />
                    </SettingGroup>
                </>
                <ExtensionStyleSettings data={data} onChange={onChange} type="categoryA1" />
            </SettingsTabs>
        </div>
    );
}
