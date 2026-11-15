/**
 * 类目导航A2 设置（对齐 view/admin mobilePro/categoryNavA2/Settings.vue）
 *
 * 页签：导航内容 / 扩展设置。
 * 内容页：内容设置（PicList）+ 参数设置（列表样式含双排滑动/每屏显示数量含自定义/
 * 图片占比宽度/缓动效果/图片填充/图片百分比/分类直角-圆角/分类间距/分类标题/底部指示条组）；
 * 扩展页：ModuleStyleEdit（无 type）+ ContentStyleEdit（type="categoryA2"，含上下/左右间距项）。
 */
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListField, DecDivider, SegmentedRadio, SettingRow } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, ContentStyleSettingsField, AnimationSettingsField, SubTitleSettingsField, PicFillTypeField } from "../../../shared/styleEdits";

export default function CategoryNavA2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const isCustom = data?.rowNum === "custom";
    const showBar = Number(data?.indicatorBar) === 1;
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["导航内容", "扩展设置"]}>
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
                                { value: "slide", label: "单列滑动" },
                                { value: "slide2", label: "双排滑动" }
                            ]}
                        />
                        <SettingRow label="每屏显示数量" block>
                            <SegmentedRadio
                                value={data?.rowNum ?? "custom"}
                                options={[
                                    { value: 2, label: "2个" },
                                    { value: 3, label: "3个" },
                                    { value: 4, label: "4个" },
                                    { value: 5, label: "5个" },
                                    { value: "custom", label: "自定义" }
                                ]}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        {isCustom ? <RangeField data={data} onChange={onChange} field="rowPicPercent" label="图片占比宽度" max={50} /> : null}
                        <AnimationSettingsField data={data} onChange={onChange} type="category_A2" />
                        <DecDivider />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" />
                        <RangeField data={data} onChange={onChange} field="picPercent" label="图片百分比" max={100} />
                        <RangeField data={data} onChange={onChange} field="radius" label="分类直角-圆角" max={200} />
                        <RangeField data={data} onChange={onChange} field="categoryPadding" label="分类间距" max={30} />
                        <SubTitleSettingsField data={data} onChange={onChange} title="分类标题" />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="indicatorBar"
                            label="底部指示条"
                            options={[
                                { value: 1, label: "滚动条" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        {showBar ? (
                            <>
                                <RangeField data={data} onChange={onChange} field="barWidth" label="指示条宽度" max={100} />
                                <RangeField data={data} onChange={onChange} field="barHeight" label="指示条高度" max={10} />
                                <RangeField data={data} onChange={onChange} field="barWidthOn" label="选中指示条宽度" max={60} />
                                <ColorField data={data} onChange={onChange} field="barColor" label="默认颜色" resetValue="#eeeeee" />
                                <ColorField data={data} onChange={onChange} field="barColorOn" label="进度颜色" resetValue="#ff6600" />
                            </>
                        ) : null}
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="categoryA2" />
                </>
            </SettingsTabs>
        </div>
    );
}
