/**
 * 图片橱窗 设置（对齐 view/admin mobilePro/imageShowcase/Settings.vue）
 *
 * 页签：图片设置（基本设置 PicList + 参数设置）/ 扩展设置
 * （ModuleStyle type="customProduct" + ContentStyle type="imageTextComponent"）。
 * 图片模式仅在 picRatio !== "default" 时出现（原版门控）。
 */
import { SettingGroup, SettingRow, RadioRowField, SegmentedRadio, RangeField, PicListField } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, ContentStyleSettingsField } from "../../../shared/styleEdits";

export default function ImageShowcaseSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["图片设置", "扩展设置"]}>
                <>
                    <SettingGroup title="基本设置">
                        <PicListField data={data} onChange={onChange} field="picList" label="" max={20} showText={false} />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="listStyle"
                            label="列表样式"
                            options={[
                                { value: "fixed", label: "固定" },
                                { value: "scroll", label: "滑动" },
                            ]}
                        />
                        <SettingRow label="排列布局">
                            <SegmentedRadio
                                value={Number(data?.rowNum ?? 1)}
                                options={[1, 2, 3, 4, 5, 6].map((n) => ({ value: n, label: `每行${n}个` }))}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picRatio"
                            label="图片比例"
                            options={[
                                { value: "default", label: "默认" },
                                { value: "3:2", label: "3:2" },
                                { value: "1:1", label: "1:1" },
                                { value: "3:4", label: "3:4" },
                                { value: "2:1", label: "2:1" },
                                { value: "16:9", label: "16:9" },
                            ]}
                        />
                        {data?.picRatio !== "default" && (
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="picFillType"
                                label="图片模式"
                                options={[
                                    { value: "cover", label: "充满" },
                                    { value: "contain", label: "周边留白" },
                                ]}
                            />
                        )}
                        <RangeField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={30} />
                        <RangeField data={data} onChange={onChange} field="imgPadding" label="图片间距" max={30} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="boxShadow"
                            label="模块投影"
                            options={[
                                { value: 0, label: "无" },
                                { value: 1, label: "常规" },
                            ]}
                        />
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="customProduct" />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="imageTextComponent" />
                </>
            </SettingsTabs>
        </div>
    );
}
