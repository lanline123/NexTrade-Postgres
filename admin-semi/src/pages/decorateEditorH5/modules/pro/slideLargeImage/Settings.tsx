/**
 * 大图滑动 设置（对齐 view/admin mobilePro/slideLargeImage/Settings.vue）
 *
 * 页签：内容设置 / 标题设置 / 扩展设置。
 * 内容页：PicList（showDesc）+ 参数（每行显示数量/滚动效果/滚动速度 max60/图片上距 max80/
 * 图片间距 max40/图片圆角 max40/图片填充）+ TitleDescEdit（picTitleDesc 嵌套编辑）；
 * 标题页：SlidingLargeImageTitle（moduleTitle 嵌套编辑）；
 * 扩展页：ModuleStyleEdit（无 type）+ ContentStyleEdit（type="slideLargeImage"，含纯色渐变项）。
 */
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListEditorField, SegmentedRadio, SettingRow, TextField } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, ContentStyleSettingsField, PicFillTypeField } from "../../../shared/styleEdits";

type NestedProps = { data: Record<string, any>; onChange: (patch: Record<string, any>) => void };
function nested(data: Record<string, any>, onChange: (patch: Record<string, any>) => void, key: string): NestedProps {
    const sub = (data?.[key] ?? {}) as Record<string, any>;
    return { data: sub, onChange: (patch: Record<string, any>) => onChange({ [key]: { ...sub, ...patch } }) };
}

const WEIGHT_OPTIONS = [
    { value: 400, label: "常规体" },
    { value: 700, label: "加粗体" }
];
const ALIGN_OPTIONS = [
    { value: "left", label: "左对齐" },
    { value: "center", label: "居中对齐" },
    { value: "right", label: "右对齐" }
];

export default function SlideLargeImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const td = nested(data, onChange, "picTitleDesc");
    const mt = nested(data, onChange, "moduleTitle");
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "标题设置", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <PicListEditorField data={data} onChange={onChange} field="picList" label="" max={20} showDesc addText="添加图片+链接" />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <SettingRow label="每行显示数量" block>
                            <SegmentedRadio
                                value={Number(data?.rowNum ?? 1.5)}
                                options={[1, 1.2, 1.5, 2, 2.2, 2.5, 3, 3.2, 3.5].map((n) => ({ value: n, label: `${n}个` }))}
                                onChange={(v) => onChange({ rowNum: v })}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="slidingEffect"
                            label="滚动效果"
                            options={[
                                { value: "auto", label: "自动滚动" },
                                { value: "manual", label: "手动滚动" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="slidingSpeed" label="滚动速度" max={60} />
                        <RangeField data={data} onChange={onChange} field="picMarginTop" label="图片上距" max={80} />
                        <RangeField data={data} onChange={onChange} field="picMargin" label="图片间距" max={40} />
                        <RangeField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={40} />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" />
                    </SettingGroup>
                    <SettingGroup title="标题描述文字">
                        <RadioRowField
                            {...td}
                            field="show"
                            label="标题描述文字"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        <ColorField {...td} field="titleColor" label="标题文字颜色" resetValue="" />
                        <RangeField {...td} field="titleFontSize" label="标题文字大小" max={20} />
                        <RadioRowField {...td} field="titleFontWeight" label="标题文字粗细" options={WEIGHT_OPTIONS} />
                        <ColorField {...td} field="descColor" label="描述文字颜色" resetValue="" />
                        <RangeField {...td} field="descFontSize" label="描述文字大小" max={20} />
                        <RadioRowField {...td} field="descFontWeight" label="描述文字粗细" options={WEIGHT_OPTIONS} />
                        <RadioRowField
                            {...td}
                            field="fontPosition"
                            label="文字展示位置"
                            options={[
                                { value: "internal", label: "图片内部" },
                                { value: "external", label: "图片外部" }
                            ]}
                        />
                        <RadioRowField {...td} field="textAlignment" label="文字对齐方式" options={ALIGN_OPTIONS} />
                        <RangeField {...td} field="marginTop" label="文字顶部距离" max={40} />
                        <RangeField {...td} field="marginBottom" label="文字底部距离" max={40} />
                        <RangeField {...td} field="marginLeft" label="文字侧边距离" max={40} />
                        <ColorField {...td} field="backgroundColor" label="内容背景颜色" resetValue="" />
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="模块标题设置">
                        <TextField {...mt} field="titleText" label="模块大标题" placeholder="请输入标题内容" />
                        <TextField {...mt} field="subTitleText" label="模块小标题" placeholder="请输入标题内容" />
                    </SettingGroup>
                    <SettingGroup title="标题样式设置">
                        <RadioRowField
                            {...mt}
                            field="titlePosition"
                            label="标题位置"
                            options={[
                                { value: "top", label: "居上" },
                                { value: "bottom", label: "居下" }
                            ]}
                        />
                        <RadioRowField {...mt} field="textAlignment" label="标题对齐方式" options={ALIGN_OPTIONS} />
                        <RangeField {...mt} field="titleMarginTop" label="标题顶部距离" max={100} />
                        <RangeField {...mt} field="titleMarginBottom" label="标题底部距离" max={100} />
                        <RangeField {...mt} field="titleMarginLeft" label="标题侧边距离" max={100} />
                        <ColorField {...mt} field="titleColor" label="大标题文字颜色" resetValue="" />
                        <RangeField {...mt} field="titleFontSize" label="大标题文字大小" min={12} max={28} />
                        <RadioRowField {...mt} field="titleFontWeight" label="大标题文字粗细" options={WEIGHT_OPTIONS} />
                        <ColorField {...mt} field="subFontTitleColor" label="小标题文字颜色" resetValue="" />
                        <RangeField {...mt} field="subFontTitleSize" label="小标题文字大小" max={20} />
                        <RadioRowField {...mt} field="subFontTitleWeight" label="小标题文字粗细" options={WEIGHT_OPTIONS} />
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} />
                    <ContentStyleSettingsField data={data} onChange={onChange} type="slideLargeImage" />
                </>
            </SettingsTabs>
        </div>
    );
}
