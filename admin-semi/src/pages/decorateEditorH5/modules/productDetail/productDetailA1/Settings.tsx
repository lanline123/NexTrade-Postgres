/**
 * 商品详情(样式A1) 设置（契约占位字段：基础文案/价格 + 显隐 + 配色）
 */
import { SettingGroup, TextField, NumberField, SwitchField, ColorField, SinglePicField, FrameSettingsField } from "../../../shared/settingWidgets";

export default function ProductDetailA1Settings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="商品信息">
                <TextField data={data} onChange={onChange} field="productTitle" label="商品标题" />
                <TextField data={data} onChange={onChange} field="description" label="商品介绍" />
                <NumberField data={data} onChange={onChange} field="price" label="售价" min={0} max={999999} suffix="元" />
                <NumberField data={data} onChange={onChange} field="marketPrice" label="市场价" min={0} max={999999} suffix="元" />
                <TextField data={data} onChange={onChange} field="buttonText" label="按钮文字" />
                <SinglePicField data={data} onChange={onChange} field="pic" label="商品图" />
            </SettingGroup>
            <SettingGroup title="显示设置">
                <SwitchField data={data} onChange={onChange} field="showName" label="显示标题" />
                <SwitchField data={data} onChange={onChange} field="showBrief" label="显示介绍" />
                <SwitchField data={data} onChange={onChange} field="showPrice" label="显示售价" />
                <SwitchField data={data} onChange={onChange} field="showMarketPrice" label="显示市场价" />
                <SwitchField data={data} onChange={onChange} field="showButton" label="显示按钮" />
            </SettingGroup>
            <SettingGroup title="颜色">
                <ColorField data={data} onChange={onChange} field="nameColor" label="标题颜色" />
                <ColorField data={data} onChange={onChange} field="priceColor" label="价格颜色" />
                <ColorField data={data} onChange={onChange} field="buttonColor" label="按钮颜色" />
                <ColorField data={data} onChange={onChange} field="backgroundColor" label="背景颜色" />
            </SettingGroup>
            <FrameSettingsField data={data} onChange={onChange} />
        </div>
    );
}
