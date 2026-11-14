/**
 * 优惠券 设置（字段覆盖渲染端 coupon/index.tsx 读取的可配置字段）
 */
import { SettingGroup, RadioRowField, TextField, ColorField, NumberField, SwitchField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function CouponSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="优惠券">
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="colorStyle"
                    label="配色样式"
                    options={[
                        { value: 1, label: "样式一" },
                        { value: 2, label: "样式二" },
                        { value: 3, label: "样式三" },
                        { value: 4, label: "样式四" }
                    ]}
                />
                <SwitchField data={data} onChange={onChange} field="showTitle" label="显示标题" />
                {Number(data?.showTitle) === 1 && (
                    <>
                        <TextField data={data} onChange={onChange} field="title" label="标题文字" />
                        <TextField data={data} onChange={onChange} field="desc" label="描述文字" />
                    </>
                )}
            </SettingGroup>
            <SettingGroup title="样式设置">
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" />
                <ColorField data={data} onChange={onChange} field="itemBackgroundColor" label="券面颜色" />
                <NumberField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxMargin" label="左右外边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxRadius" label="圆角" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="itemPadding" label="券内边距" max={50} suffix="px" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
