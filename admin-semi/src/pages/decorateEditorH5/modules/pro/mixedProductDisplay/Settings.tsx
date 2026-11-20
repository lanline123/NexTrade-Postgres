/**
* 商品混排 设置（对齐 view/admin mobilePro/mixedProductDisplay/Settings.vue）
*
* 页签：商品设置（ComboGroupEdit 分组编辑 / 右侧小图 / 倒计时设置 / 参数设置）
* / 扩展设置（颜色设置 8 色 + ModuleStyle/ContentStyle type="mixedProductDisplay"）。
* 分组内商品选择走共享 ProductInfoSelect（快照式选品），可增删/更换/拖拽排序各组 pruductList（字段名沿原版拼写）。
* 查看更多链接 SelectLink 以文本输入承接，写回 moreLink 同一嵌套结构。
* 时间选择：el-time-picker/el-date-picker 以原生 time/datetime-local 输入承接，写回时间戳数组/数值。
* 商品销量 productSales 在原版 Settings 中被注释、不生效，故不提供编辑入口（defaults 保留字段）。
*/
import { Checkbox, CheckboxGroup, DatePicker, Input, TimePicker } from "@douyinfe/semi-ui";
import {
SettingGroup,
SettingRow,
RadioRowField,
RangeField,
ColorField,
SinglePicField,
DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings, PicFillTypeField} from "../../../shared/styleEdits";
import ProductInfoSelect from "../../../shared/ProductInfoSelect";
import { useI18n} from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
data: Record<string, any>;
onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"}
];

const toTimeValue = (ts: number): string => {
if (!ts) return "";
const d = new Date(ts);
return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};
const fromTimeValue = (base: number, v: string): number => {
const [h, m] = v.split(":").map(Number);
const d = new Date(base || Date.now());
d.setHours(h || 0, m || 0, 0, 0);
return d.getTime();
};
const toDateTimeValue = (ts: number): string => {
if (!ts) return "";
const d = new Date(ts);
const p = (n: number) => String(n).padStart(2, "0");
return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
};

function GroupEditor({ data, onChange}: Props) {
const { t} = useI18n();
const groups: Array<Record<string, any>> = Array.isArray(data?.productGroups)? data.productGroups: [];
const setGroups = (next: Array<Record<string, any>>) => onChange({ productGroups: next});
const patchGroup = (index: number, patch: Patch) => setGroups(groups.map((g, i) => (i === index? {...g,...patch}: g)));
return (
<>
{groups.map((group, index) => {
const linkUrl = group.moreLink?.link?.link?? "";
return (
<SettingGroup key={index} title={`第${index + 1}组`}>
<SettingRow label="分组大标题">
<Input
value={group.groupTitle?? ""}
placeholder={t("请输入标题内容")}
style={{ width: 240, maxWidth: "100%"}}
onChange={(v: string) => patchGroup(index, { groupTitle: v})}
/>
</SettingRow>
<SettingRow label="分组小标题">
<Input
value={group.groupSubTitle?? ""}
placeholder={t("请输入标题内容")}
style={{ width: 240, maxWidth: "100%"}}
onChange={(v: string) => patchGroup(index, { groupSubTitle: v})}
/>
</SettingRow>
<SettingRow label="查看更多链接">
<Input
value={linkUrl}
placeholder={t("跳转链接，如 /pages/product/index?id=1")}
style={{ width: 240, maxWidth: "100%"}}
onChange={(v: string) =>
patchGroup(index, {
moreLink: {...(group.moreLink || {}), link: {...(group.moreLink?.link || {}), link: v}}
})
}
/>
</SettingRow>
<SettingRow label="选择商品" block>
<ProductInfoSelect
value={Array.isArray(group.pruductList) ? group.pruductList : []}
onChange={(next) => patchGroup(index, { pruductList: next })}
isMultiple
/>
</SettingRow>
</SettingGroup>
);
})}
</>
);
}

export default function MixedProductDisplaySettings({ data, onChange}: Props) {
const { t} = useI18n();
const moduleColor = data?.moduleColor?? {};
const setColor = (patch: Patch) => onChange({ moduleColor: {...moduleColor,...patch}});
const colorProps: Props = { data: moduleColor, onChange: setColor};

const dailyTime: number[] = Array.isArray(data?.dailyTime)? data.dailyTime: [];
const timeContent: string[] = Array.isArray(data?.timeContent)? data.timeContent: [];
const setTimeContent = (values: string[]) => {
const order = ["时", "分", "秒"];
onChange({ timeContent: [...values].sort((x, y) => order.indexOf(x) - order.indexOf(y))});
};

return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["商品设置", "扩展设置"]}>
<>
<GroupEditor data={data} onChange={onChange} />
<SinglePicField data={data} onChange={onChange} field="rightPic" label="右侧: 自定小图: 宽200内, 高48" />
<SettingGroup title="倒计时设置">
<RadioRowField data={data} onChange={onChange} field="showCountdown" label="倒计时" options={SHOW_OPTIONS} />
{Number(data?.showCountdown) === 1? (
<RadioRowField
data={data}
onChange={onChange}
field="countdownType"
label="倒计时类型"
options={[
{ value: "dailyCycle", label: "每日循环"},
{ value: "specifyTime", label: "指定时间"}
]}
/>
): null}
{data?.countdownType === "dailyCycle" && Number(data?.showCountdown) === 1? (
<SettingRow label="每日活动开始时间" block>
<div style={{ display: "flex", alignItems: "center", gap: 8}}>
<TimePicker
format="HH:mm"
value={dailyTime[0] ? new Date(dailyTime[0]) : undefined}
style={{ width: 108 }}
onChange={(_time, timeString) => { if (timeString) onChange({ dailyTime: [fromTimeValue(dailyTime[0], String(timeString)), dailyTime[1] ?? 0] }); }}
/>
<span>{t("至")}</span>
<TimePicker
format="HH:mm"
value={dailyTime[1] ? new Date(dailyTime[1]) : undefined}
style={{ width: 108 }}
onChange={(_time, timeString) => { if (timeString) onChange({ dailyTime: [dailyTime[0] ?? 0, fromTimeValue(dailyTime[1], String(timeString))] }); }}
/>
</div>
</SettingRow>
): null}
{data?.countdownType === "specifyTime" && Number(data?.showCountdown) === 1? (
<SettingRow label="每日活动开始时间" block>
<DatePicker
type="dateTime"
format="yyyy-MM-dd HH:mm"
value={data?.specifyTime ? new Date(Number(data.specifyTime)) : undefined}
style={{ width: 240, maxWidth: "100%" }}
onChange={(date) => onChange({ specifyTime: date ? new Date(date as Date).getTime() : 0 })}
/>
</SettingRow>
): null}
{Number(data?.showCountdown) === 1? (
<SettingRow label="选择显示内容" block>
<CheckboxGroup value={timeContent} onChange={(v: any) => setTimeContent(v as string[])}>
<Checkbox value="时">{t("时")}</Checkbox>
<Checkbox value="分">{t("分")}</Checkbox>
<Checkbox value="秒">{t("秒")}</Checkbox>
</CheckboxGroup>
</SettingRow>
): null}
</SettingGroup>
<SettingGroup title="参数设置">
<RadioRowField
data={data}
onChange={onChange}
field="picScale"
label="图片比例"
options={[
{ value: "3:2", label: "3:2"},
{ value: "1:1", label: "1:1"},
{ value: "3:4", label: "3:4"},
{ value: "16:9", label: "16:9"}
]}
/>
<PicFillTypeField data={data} onChange={onChange} field="picFillType" label="图片填充" />
<RangeField data={data} onChange={onChange} field="productRadius" label="商品圆角" max={40} />
<RadioRowField
data={data}
onChange={onChange}
field="morePosition"
label="更多链接占位"
options={[
{ value: "top", label: "顶部标题区"},
{ value: "auto", label: "标题+商品区"}
]}
/>
</SettingGroup>
<DecDivider />
<RadioRowField data={data} onChange={onChange} field="showProductPrice" label="商品价格" options={SHOW_OPTIONS} />
{Number(data?.showProductPrice) === 1? (
<RadioRowField
data={data}
onChange={onChange}
field="priceWeight"
label="商品价格粗细"
options={[
{ value: 400, label: "常规体"},
{ value: 700, label: "加粗体"}
]}
/>
): null}
<RadioRowField data={data} onChange={onChange} field="crossedOutPrice" label="商品划线价" options={SHOW_OPTIONS} />
</>
<>
<SettingGroup title="颜色设置">
<ColorField {...colorProps} field="titleColor" label="模块大标题颜色" resetValue="" />
<ColorField {...colorProps} field="sellingPointColor" label="卖点小标题颜色" resetValue="" />
<ColorField {...colorProps} field="countdownBackgroundColor" label="倒计时背景颜色" resetValue="" />
<ColorField {...colorProps} field="countdownColor" label="倒计时数字颜色" resetValue="" />
</SettingGroup>
<DecDivider />
<ColorField {...colorProps} field="priceBackgroundColor" label="价格背景颜色" resetValue="" />
<ColorField {...colorProps} field="priceColor" label="商品价格颜色" resetValue="" />
<ColorField {...colorProps} field="crossedOutPriceColor" label="商品划线价颜色" resetValue="" />
<ColorField {...colorProps} field="lineColor" label="中间分割线颜色" resetValue="" />
<ExtensionStyleSettings data={data} onChange={onChange} type="mixedProductDisplay" />
</>
</SettingsTabs>
</div>
);
}
