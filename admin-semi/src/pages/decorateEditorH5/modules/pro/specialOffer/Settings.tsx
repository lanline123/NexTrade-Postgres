/**
 * 特价专区 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, TextField, RadioRowField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function SpecialOfferSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const mt = data?.moduleTitle ?? {}; const setMt = (p: Record<string, any>) => onChange({ moduleTitle: { ...mt, ...p } });
    const mc = data?.moduleColor ?? {}; const setMc = (p: Record<string, any>) => onChange({ moduleColor: { ...mc, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="标题区">
                <TextField data={mt} onChange={setMt} field="title" label="标题" />
                <TextField data={mt} onChange={setMt} field="subTitle" label="副标题" />
                <TextField data={mt} onChange={setMt} field="buttonText" label="按钮文字" />
                <TextField data={mt} onChange={setMt} field="countdownTitle" label="倒计时标题" />
                <RadioRowField data={mt} onChange={setMt} field="countdownType" label="倒计时类型" options={[{ value: "dailyCycle", label: "每日循环" }, { value: "specifyTime", label: "指定时间" }]} />
                <ColorField data={mc} onChange={setMc} field="titleColor" label="标题颜色" />
                <ColorField data={mc} onChange={setMc} field="countdownColor" label="倒计时颜色" />
                <ColorField data={mc} onChange={setMc} field="buttonTextColor" label="按钮颜色" />
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="商品样式">
                <NumberField data={data} onChange={onChange} field="productWidth" label="卡片宽度" min={20} max={100} suffix="%" />
                <NumberField data={data} onChange={onChange} field="productRadius" label="卡片圆角" max={30} suffix="px" />
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showProductPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="showSellingPoints" label="显示卖点" />
                <SwitchField data={data} onChange={onChange} field="showProdcutTitles" label="显示商品标题组" />
                <ColorField data={mc} onChange={setMc} field="productNameColor" label="商品名颜色" />
                <ColorField data={mc} onChange={setMc} field="sellingPointsColor" label="卖点颜色" />
            </SettingGroup>
        </div>
    );
}
