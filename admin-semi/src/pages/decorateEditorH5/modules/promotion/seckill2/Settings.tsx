/**
 * 秒杀2 设置（对齐 view/admin seckill2/Settings.vue；原版手写字段，非 CommonActivityFrameEdit）
 *
 * 选择模板（1 一行一个/2 一行两个/3 一行三个/10 横向滑动）→ 图片背景色（重置 #ffffff）→
 * 商品信息边距 → 显示商品名称 → 显示商品价格 → 商品边距（max 50）→ 背景颜色（重置 #ffffff）→
 * 圆角/内边距/左右边距/上边距/下边距（max 30）→ CommonTitleEdit type="seckill2"
 * （TitleSettingsField titleType="seckill2"，含标题右倒计时相关项）。
 */
import { SettingGroup, SegmentedRadio, ColorField, RangeField, DecDivider, TitleSettingsField, RadioRowField } from "../../../shared/settingWidgets";

const SHOW = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

export default function Seckill2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup>
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="style"
                    label="选择模板"
                    options={[
                        { value: 1, label: "一行一个" },
                        { value: 2, label: "一行两个" },
                        { value: 3, label: "一行三个" },
                        { value: 10, label: "横向滑动" }
                    ]}
                />
            </SettingGroup>
            <DecDivider />
            <SettingGroup>
                <ColorField data={data} onChange={onChange} field="imageBackgroundColor" label="图片背景色" resetValue="#ffffff" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsNamePadding"
                    label="商品信息边距"
                    options={[
                        { value: 1, label: "有边距" },
                        { value: 0, label: "无边距" }
                    ]}
                />
                <RadioRowField data={data} onChange={onChange} field="showName" label="显示商品名称" options={SHOW} />
                <RadioRowField data={data} onChange={onChange} field="showPrice" label="显示商品价格" options={SHOW} />
                <RangeField data={data} onChange={onChange} field="goodsPadding" label="商品边距" max={50} />
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" resetValue="#ffffff" />
                <RangeField data={data} onChange={onChange} field="boxRadius" label="圆角" max={30} />
                <RangeField data={data} onChange={onChange} field="innerPadding" label="内边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={30} />
                <RangeField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={30} />
            </SettingGroup>
            <TitleSettingsField data={data} onChange={onChange} field="title" titleType="seckill2" />
        </div>
    );
}
