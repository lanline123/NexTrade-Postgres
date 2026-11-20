/**
 * 小图滑动 设置（对齐 view/admin mobilePro/slidingImage/Settings.vue）
 *
 * 页签：内容设置 / 扩展设置。
 * 内容页：内容设置（PicList 含描述）+ 图片样式设置（背景渐变方向/图片宽度 max50/图片间距 max20/
 * 图片内距 max10/图片圆角 max40/缩小比例 max1 step0.01/标题/描述文字及颜色）；
 * 扩展页：ModuleStyleEdit（无 type）+「轮播扩展设置」组（自动轮播/轮播时间间隔 max10）。
 * 注：原版 PicList 带 showColor（逐项渐变色 gradientColorA/B），我方 PicListField 无逐项颜色编辑，
 * 逐项渐变色保持原数据不变（见交付摘要）。
 */
import { SettingGroup, RadioRowField, RangeField, ColorField, PicListEditorField } from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, SubTitleSettingsField } from "../../../shared/styleEdits";

export default function SlidingImageSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <PicListEditorField data={data} onChange={onChange} field="picList" label="" max={20} showDesc addText="添加图片+链接" />
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
