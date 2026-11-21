/**
 * 小图滑动 设置（对齐 view/admin mobilePro/slidingImage/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置。
 * 内容页：内容设置（PicList 含描述）+ 图片样式设置（背景渐变方向/图片宽度 max50/图片间距 max20/
 * 图片内距 max10/图片圆角 max40/缩小比例 max1 step0.01/标题/描述文字及颜色）；
 * 扩展页：ModuleStyleEdit（无 type）+「轮播扩展设置」组（自动轮播/轮播时间间隔 max10）。
 * 逐项渐变色（2026-11-21 修复）：原版 PicList 带 showColor（逐项渐变色 gradientColorA/B 的
 * 选色器 + 添加时写键），我方此前无此形态——新加条目永无渐变色、渲染端无条件消费该字段，
 * 卡片无背景「没样式」且无处可改。现 PicListEditorField 开 showColor：编辑弹窗内可逐项
 * 改背景色，新条目按默认调色板（slidingImageDefaults 四对）播种，与默认条目同形状。
 */
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListEditorField } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, SubTitleSettingsField } from "../../../shared/styleEdits";
import { slidingImageDefaults } from "./index";

/** 逐项渐变色播种调色板 = 模块默认四条目的 [A, B] 对（与 defaults 同源，新条目按下标循环取） */
const COLOR_PALETTE: Array<[string, string]> = slidingImageDefaults.picList.map(
    (item) => [item.gradientColorA, item.gradientColorB] as [string, string]
);

export default function SlidingImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <PicListEditorField data={data} onChange={onChange} field="picList" label="" max={20} showDesc showColor colorPalette={COLOR_PALETTE} addText="添加图片+链接" />
                    </SettingGroup>
                    <SettingGroup title="图片样式设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="backgroundGradientType"
                            label="背景渐变方向"
                            options={[
                                { value: "upDown", label: "上下" },
                                { value: "leftRight", label: "左右" },
                                { value: "diagonal", label: "斜向" },
                                { value: "purity", label: "单色" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="picWidth" label="图片宽度" max={50} />
                        <RangeField data={data} onChange={onChange} field="picMargin" label="图片间距" max={20} />
                        <RangeField data={data} onChange={onChange} field="picPadding" label="图片内距" max={10} />
                        <RangeField data={data} onChange={onChange} field="picRadius" label="图片圆角" max={40} />
                        <RangeField data={data} onChange={onChange} field="picReducePercent" label="缩小比例" min={0} max={1} step={0.01} />
                        <SubTitleSettingsField data={data} onChange={onChange} title="标题" />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="showDesc"
                            label="描述文字"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        {Number(data?.showDesc) === 1 ? (
                            <ColorField data={data} onChange={onChange} field="descColor" label="描述文字颜色" resetValue="#ffffff" />
                        ) : null}
                    </SettingGroup>
                </>
                <>
                    <ModuleStyleSettingsField data={data} onChange={onChange} />
                    <SettingGroup title="轮播扩展设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="autoPlay"
                            label="自动轮播"
                            options={[
                                { value: 1, label: "开启" },
                                { value: 0, label: "关闭" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="interval" label="轮播时间间隔" max={10} />
                    </SettingGroup>
                </>
            </SettingsTabs>
        </div>
    );
}
