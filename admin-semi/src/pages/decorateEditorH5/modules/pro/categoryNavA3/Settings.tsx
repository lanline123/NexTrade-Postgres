/**
 * 类目导航A3 设置（对齐 view/admin mobilePro/categoryNavA3/Settings.vue）
 *
 * 页签：导航内容 / 扩展设置。原版 Settings 标题 h3 误写「类目导航A1」（原版笔误），此处按模块实际名呈现。
 * 内容页：内容设置（PicList，showDesc=true）+ 参数设置（列表样式/每屏显示数量/图片背景色/
 * 图片填充/图片百分比/分类直角-圆角/分类间距/分类标题/图标设置 PicSelect）；
 * 扩展页：ModuleStyleEdit + ContentStyleEdit（type="categoryA1"）。
 */
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListField, PicSelectField, DecDivider, SegmentedRadio, SettingRow } from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings, SubTitleSettingsField, PicFillTypeField } from "../../../shared/styleEdits";
import { defaultFrame } from "../../../contract/sharedDefaults";

export default function CategoryNavA3Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["导航内容", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <PicListField data={data} onChange={onChange} field="categoryList" label="" max={30} showDesc addText="添加一组" />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="listStyleType"
                            label="列表样式"
                            options={[
                                { value: "list", label: "固定" },
                                { value: "slide", label: "单列滑动" }
                            ]}
                        />
                        <SettingRow label="每屏显示数量" block>
                            <SegmentedRadio
                                value={Number(data?.rowNum ?? 4)}
                                options={[2, 3, 4, 5, 6].map((n) => ({ value: n, label: `${n}个` }))}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        <DecDivider />
                        <ColorField data={data} onChange={onChange} field="imageBackgroundColor" label="图片背景色" resetValue={defaultFrame.backgroundColor} />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" />
                        <RangeField data={data} onChange={onChange} field="picPercent" label="图片百分比" max={100} />
                        <RangeField data={data} onChange={onChange} field="radius" label="分类直角-圆角" max={200} />
                        <RangeField data={data} onChange={onChange} field="categoryPadding" label="分类间距" max={30} />
                        <SubTitleSettingsField data={data} onChange={onChange} title="分类标题" />
                        <PicSelectField data={data} onChange={onChange} field="logoPic" label="图标设置：宽16高16" />
                    </SettingGroup>
                </>
                <ExtensionStyleSettings data={data} onChange={onChange} type="categoryA1" />
            </SettingsTabs>
        </div>
    );
}
