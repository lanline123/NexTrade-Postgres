/**
 * 倒计时 设置
 */
import { SettingGroup, NumberField, ColorField, SwitchField, TextField, RadioRowField, SelectField } from "../../../shared/settingWidgets";
import ProductsSourceField from "../../../shared/ProductsSourceField";

export default function CountdownSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const cd = data?.countdownData ?? {}; const setCd = (p: Record<string, any>) => onChange({ countdownData: { ...cd, ...p } });
    const mt = data?.moduleTitle ?? {}; const setMt = (p: Record<string, any>) => onChange({ moduleTitle: { ...mt, ...p } });
    const mc = data?.moduleColor ?? {}; const setMc = (p: Record<string, any>) => onChange({ moduleColor: { ...mc, ...p } });
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title="标题">
                <TextField data={mt} onChange={setMt} field="title" label="标题" />
                <TextField data={mt} onChange={setMt} field="subTitle" label="副标题" />
                <TextField data={mt} onChange={setMt} field="moreText" label="右侧文字" />
                <ColorField data={mc} onChange={setMc} field="titleColor" label="标题颜色" />
                <ColorField data={mc} onChange={setMc} field="subTitleColor" label="副标题颜色" />
            </SettingGroup>
            <SettingGroup title="倒计时">
                <SwitchField data={cd} onChange={setCd} field="showCountdown" label="显示倒计时" />
                <RadioRowField data={cd} onChange={setCd} field="countdownType" label="倒计时类型" options={[{ value: "dailyCycle", label: "每日循环" }, { value: "specifyTime", label: "指定时间" }]} />
                <NumberField data={cd} onChange={setCd} field="specifyTime" label="结束时间戳" max={9999999999999} />
                <ColorField data={mc} onChange={setMc} field="countdownColor" label="倒计时颜色" />
            </SettingGroup>
            <SettingGroup title="商品来源">
                <ProductsSourceField data={data} onChange={onChange} />
            </SettingGroup>
            <SettingGroup title="商品样式">
                <NumberField data={data} onChange={onChange} field="rowNum" label="展示数量" min={1} max={6} />
                <SwitchField data={data} onChange={onChange} field="showProductName" label="显示商品名" />
                <SwitchField data={data} onChange={onChange} field="showProductPrice" label="显示价格" />
                <SwitchField data={data} onChange={onChange} field="crossedOutPrice" label="显示划线价" />
                <SelectField data={data} onChange={onChange} field="picFillType" label="图片填充" options={[{ value: "cover", label: "填充" }, { value: "contain", label: "留白" }]} />
                <ColorField data={mc} onChange={setMc} field="productPriceColor" label="价格颜色" />
                <ColorField data={mc} onChange={setMc} field="productNameColor" label="商品名颜色" />
            </SettingGroup>
        </div>
    );
}
