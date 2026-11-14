/**
 * 秒杀（样式2） 设置（字段覆盖渲染端 seckill2/index.tsx 读取的可配置字段 + 商品来源）
 */
import { SettingGroup, RadioRowField, ColorField, NumberField, SwitchField, TitleSettingsField, FrameSettingsField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

const STYLE_OPTIONS = [
    { value: 1, label: "一行一个" },
    { value: 2, label: "一行两个" },
    { value: 3, label: "一行三个" },
    { value: 4, label: "一大两小" },
    { value: 5, label: "详细列表" },
    { value: 6, label: "横向滑动" },
    { value: 10, label: "横滑三列" }
];

export default function Seckill2Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="秒杀商品">
                <RadioRowField data={data} onChange={onChange} field="style" label="选择模板" options={STYLE_OPTIONS} />
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="显示设置">
                <SwitchField data={data} onChange={onChange} field="showName" label="显示商品名称" />
                <SwitchField data={data} onChange={onChange} field="showPrice" label="显示价格" />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsStyle"
                    label="商品样式"
                    options={[
                        { value: 1, label: "无边白底" },
                        { value: 2, label: "卡片投影" },
                        { value: 3, label: "描边白底" },
                        { value: 4, label: "无边透明底" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="goodsRadioStyle"
                    label="商品倒角"
                    options={[
                        { value: 1, label: "直角" },
                        { value: 2, label: "圆角" }
                    ]}
                />
                <RadioRowField
                    data={data}
                    onChange={onChange}
                    field="textAlign"
                    label="文本对齐"
                    options={[
                        { value: 1, label: "居左" },
                        { value: 2, label: "居中" }
                    ]}
                />
            </SettingGroup>
            <SettingGroup title="扩展设置">
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" />
                <ColorField data={data} onChange={onChange} field="imageBackgroundColor" label="图片背景色" />
                <NumberField data={data} onChange={onChange} field="boxRadius" label="圆角" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="innerPadding" label="内边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPadding" label="左右边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingTop" label="上边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="boxPaddingBottom" label="下边距" max={50} suffix="px" />
                <NumberField data={data} onChange={onChange} field="goodsPadding" label="商品间距" max={50} suffix="px" />
            </SettingGroup>
            <TitleSettingsField data={data} onChange={onChange} label="模块标题" />
            <FrameSettingsField data={data} onChange={onChange} label="边框设置" />
        </div>
    );
}
