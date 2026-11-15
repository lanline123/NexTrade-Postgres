/**
 * 优惠券 设置（对齐 view/admin coupon/Settings.vue；原版无页签、无 frame 编辑组）
 *
 * 显示标题栏（1 显示/0 不显示）→ 标题/描述（仅显示时，描述 textarea maxlength 500）→
 * 优惠券颜色色板单选（1 红 #e74c2c / 2 黄 #ffe9b7 / 3 黑 #383b3e / 4 绿 #e0f4e4，写 colorStyle）→
 * 组件背景 itemBackgroundColor（重置 = defaultFrame.itemBackgroundColor 即空）→
 * 背景颜色 backgroundColor（重置 = defaultFrame.backgroundColor #ffffff）→
 * 滑杆 max 30：组件边距 itemPadding / 圆角 boxRadius / 左右边距 boxPadding / 左右外间距 boxMargin / 上边距 / 下边距。
 */
import { TextArea } from "@douyinfe/semi-ui";
import { SettingGroup, SettingRow, SegmentedRadio, TextField, ColorField, RangeField, RadioRowField } from "../../../shared/settingWidgets";

const COLOR_SWATCHES = [
    { value: 1, label: "红色", color: "#e74c2c" },
    { value: 2, label: "黄色", color: "#ffe9b7" },
    { value: 3, label: "黑色", color: "#383b3e" },
    { value: 4, label: "绿色", color: "#e0f4e4" }
];

export default function CouponSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const showTitle = data?.showTitle ?? 0;
    const colorStyle = data?.colorStyle ?? 0;
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="showTitle"
                    label="显示标题栏"
                    options={[
                        { value: 1, label: "显示" },
                        { value: 0, label: "不显示" }
                    ]}
                />
                {showTitle === 1 ? (
                    <>
                        <TextField data={data} onChange={onChange} field="title" label="标题内容" placeholder="请输入标题内容" />
                        <SettingRow label="描述内容" block>
                            <TextArea
                                value={data?.desc ?? ""}
                                maxCount={500}
                                rows={3}
                                placeholder="请输入描述内容, 字数不超过500"
                                onChange={(v: string) => onChange({ desc: v.slice(0, 500) })}
                            />
                        </SettingRow>
                    </>
                ) : null}
                <SettingRow label="优惠券颜色" block>
                    <div style={{ display: "flex", gap: 10, padding: "2px 0" }}>
                        {COLOR_SWATCHES.map((s) => (
                            <button
                                key={s.value}
                                type="button"
                                title={s.label}
                                onClick={() => onChange({ colorStyle: s.value })}
                                style={{
                                    width: 40,
                                    height: 28,
                                    padding: 3,
                                    borderRadius: 4,
                                    cursor: "pointer",
                                    background: "#fff",
                                    border: colorStyle === s.value ? "1px solid var(--semi-color-primary, #155bd4)" : "1px solid #e5e5e5"
                                }}
                            >
                                <span style={{ display: "block", width: "100%", height: "100%", borderRadius: 2, backgroundColor: s.color }} />
                            </button>
                        ))}
                    </div>
                </SettingRow>
                <ColorField data={data} onChange={onChange} field="itemBackgroundColor" label="组件背景" resetValue="" />
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue="#ffffff" />
                <RangeField data={data} onChange={onChange} field="itemPadding" label="组件边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxRadius" label="圆角" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxMargin" label="左右外间距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} />
            </SettingGroup>
        </div>
    );
}
