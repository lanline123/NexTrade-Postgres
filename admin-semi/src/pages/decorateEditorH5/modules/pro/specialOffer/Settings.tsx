/**
 * 天天特价 设置（对齐 view/admin mobilePro/specialOffer/Settings.vue）
 *
 * 页签：商品设置 / 扩展设置（颜色设置 + ExtensionStyleSettings type="specialOffer"）。
 * - 商品来源：ProductInfoSelect / ProductGroupSelect 均未移植——手动商品保留演示项渲染 +
 *   只读提示，分组保留已选数量只读展示（customProduct 口径）；
 * - 模块颜色风格切换的联动与原版 resetColor 一致：写回 moduleColor.countdownTitleColor 与
 *   contentStyle.gradientColorA/B 为 ModuleTheme 预设；
 * - prodcutTitles 逐项编辑：按原版 pushProductTitle 语义补齐（手动=productList 长度，
 *   分组=6），新项默认「自定义大标题/自定义卖点」，键名 title{n}/sellingPoints{n} 逐字保留；
 * - 按钮链接 SelectLink 以文本输入承接，写回 buttonLink.link.link 同一嵌套结构；
 * - 每日时间以原生 time 输入承接（原版 el-time-picker is-range），写回时间戳数组。
 */
import { Input } from "@douyinfe/semi-ui";
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RadioField,
    RangeField,
    ColorField,
    TextField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs, ExtensionStyleSettings, PicFillTypeField } from "../../../shared/styleEdits";
import { SPECIAL_OFFER_THEMES } from "./Preview";
import { useI18n } from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];
const WEIGHT_OPTIONS = [
    { value: 400, label: "常规体" },
    { value: 700, label: "加粗体" }
];

function toTimeInput(ts: number): string {
    if (!ts) return "";
    const d = new Date(ts);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
function timeInputToTs(base: number, value: string): number {
    const [h, m] = value.split(":").map((x) => Number(x) || 0);
    const d = new Date(base || Date.now());
    d.setHours(h, m, 0, 0);
    return d.getTime();
}
function toDatetimeInput(ts: number): string {
    if (!ts) return "";
    const d = new Date(ts);
    const p = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

export default function SpecialOfferSettings({ data, onChange }: Props) {
    const { t } = useI18n();
    const mt = data?.moduleTitle ?? {};
    const mtProps: Props = { data: mt, onChange: (patch: Patch) => onChange({ moduleTitle: { ...mt, ...patch } }) };
    const mc = data?.moduleColor ?? {};
    const mcProps: Props = { data: mc, onChange: (patch: Patch) => onChange({ moduleColor: { ...mc, ...patch } }) };

    const productList: Array<Record<string, any>> = Array.isArray(data?.productList) ? data.productList : [];
    const productGroups: Array<Record<string, any>> = Array.isArray(data?.productGroups) ? data.productGroups : [];
    const titlesCount = data?.productSource === "productGroup" ? 6 : productList.length;
    const prodcutTitles: Array<Record<string, any>> = Array.isArray(data?.prodcutTitles) ? data.prodcutTitles : [];
    const patchTitle = (index: number, key: string, value: string) => {
        const next = [...prodcutTitles];
        while (next.length <= index) {
            const i = next.length;
            next.push({ [`title${i + 1}`]: "自定义大标题", [`sellingPoints${i + 1}`]: "自定义卖点" });
        }
        next[index] = { ...next[index], [key]: value };
        onChange({ prodcutTitles: next });
    };

    const daily: number[] = Array.isArray(mt.dailyTime) ? mt.dailyTime : [];
    const btnLinkUrl = mt.buttonLink?.link?.link ?? (typeof mt.buttonLink?.link === "string" ? mt.buttonLink.link : "");

    const changeColorStyle = (style: string) => {
        const theme = SPECIAL_OFFER_THEMES[style];
        const patch: Patch = { moduleColorStyle: style };
        if (theme) {
            patch.moduleColor = { ...mc, countdownTitleColor: theme.textColor };
            patch.contentStyle = { ...(data?.contentStyle ?? {}), gradientColorA: theme.gradientColorA, gradientColorB: theme.gradientColorB };
        }
        onChange(patch);
    };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["商品设置", "扩展设置"]}>
                <>
                    <SettingGroup title="商品设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="productSource"
                            label="商品来源"
                            options={[
                                { value: "products", label: "手动选择" },
                                { value: "productGroup", label: "商品分组" }
                            ]}
                        />
                        {data?.productSource === "products" ? (
                            <SettingRow label="选择商品" block>
                                <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6 }}>
                                    商品选择器（ProductInfoSelect 数据选择组件）尚未接入本编辑器，当前已选/演示商品 {productList.length} 件，数据保持不变。
                                    <br />
                                    {t("提示：您可以通过拖拽进行商品排序")}
                                </div>
                            </SettingRow>
                        ) : (
                            <SettingRow label="商品分组" block>
                                <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6 }}>
                                    商品分组选择器（ProductGroupSelect 数据选择组件）尚未接入本编辑器，当前已选分组 {productGroups.length} 个，数据保持不变。
                                </div>
                            </SettingRow>
                        )}
                        <SettingRow label="模块颜色风格" block>
                            <RadioField
                                data={data}
                                onChange={(patch: Patch) => {
                                    if (patch.moduleColorStyle) changeColorStyle(String(patch.moduleColorStyle));
                                    else onChange(patch);
                                }}
                                field="moduleColorStyle"
                                options={[
                                    { value: "orange", label: "番茄橙" },
                                    { value: "red", label: "中国红" },
                                    { value: "blue", label: "天空蓝" },
                                    { value: "green", label: "森林绿" },
                                    { value: "purple", label: "紫罗兰" },
                                    { value: "grey", label: "深灰色" },
                                    { value: "chocolate", label: "巧克力" },
                                    { value: "brown", label: "马鞍棕" }
                                ]}
                            />
                        </SettingRow>
                    </SettingGroup>
                    {Number(data?.showProdcutTitles) === 1 && titlesCount > 0 ? (
                        <SettingGroup title="自定义标题/卖点">
                            {Array.from({ length: titlesCount }, (_, index) => (
                                <div key={index}>
                                    <SettingRow label={`商品${index + 1}标题`}>
                                        <Input
                                            value={prodcutTitles[index]?.[`title${index + 1}`] ?? ""}
                                            placeholder={t("请输入标题内容")}
                                            style={{ width: 240, maxWidth: "100%" }}
                                            onChange={(v: string) => patchTitle(index, `title${index + 1}`, v)}
                                        />
                                    </SettingRow>
                                    <SettingRow label={`商品${index + 1}卖点`}>
                                        <Input
                                            value={prodcutTitles[index]?.[`sellingPoints${index + 1}`] ?? ""}
                                            placeholder={t("请输入卖点内容")}
                                            style={{ width: 240, maxWidth: "100%" }}
                                            onChange={(v: string) => patchTitle(index, `sellingPoints${index + 1}`, v)}
                                        />
                                    </SettingRow>
                                </div>
                            ))}
                        </SettingGroup>
                    ) : null}
                    <SettingGroup title="模块标题设置">
                        <TextField {...mtProps} field="title" label="模块大标题" placeholder="请输入标题内容" />
                        <TextField {...mtProps} field="subTitle" label="模块小标题" placeholder="请输入标题内容" />
                        <RadioRowField
                            {...mtProps}
                            field="topRightType"
                            label="顶部右侧展示"
                            options={[
                                { value: "countdown", label: "倒计时" },
                                { value: "button", label: "按钮" }
                            ]}
                        />
                        {mt.topRightType === "button" ? (
                            <>
                                <TextField {...mtProps} field="buttonText" label="按钮文案" placeholder="请输入按钮文案" />
                                <SettingRow label="按钮跳转链接">
                                    <Input
                                        value={btnLinkUrl}
                                        placeholder={t("跳转链接，如 /pages/product/index?id=1")}
                                        style={{ width: 240, maxWidth: "100%" }}
                                        onChange={(v: string) =>
                                            mtProps.onChange({
                                                buttonLink: { ...(mt.buttonLink || {}), link: { ...(mt.buttonLink?.link && typeof mt.buttonLink.link === "object" ? mt.buttonLink.link : {}), link: v } }
                                            })
                                        }
                                    />
                                </SettingRow>
                            </>
                        ) : (
                            <>
                                <TextField {...mtProps} field="countdownTitle" label="倒计时文案" placeholder="请输入倒计时文案" />
                                <RadioRowField
                                    {...mtProps}
                                    field="countdownType"
                                    label="倒计时类型"
                                    options={[
                                        { value: "dailyCycle", label: "每日循环" },
                                        { value: "specifyTime", label: "指定时间" }
                                    ]}
                                />
                                {mt.countdownType === "specifyTime" ? (
                                    <SettingRow label="活动结束时间">
                                        <input
                                            type="datetime-local"
                                            value={toDatetimeInput(Number(mt.specifyTime) || 0)}
                                            onChange={(e) => mtProps.onChange({ specifyTime: new Date(e.target.value).getTime() || 0 })}
                                        />
                                    </SettingRow>
                                ) : (
                                    <>
                                        <SettingRow label="每日开始时间">
                                            <input type="time" value={toTimeInput(Number(daily[0]) || 0)} onChange={(e) => mtProps.onChange({ dailyTime: [timeInputToTs(Number(daily[0]) || 0, e.target.value), daily[1] ?? 0] })} />
                                        </SettingRow>
                                        <SettingRow label="每日结束时间">
                                            <input type="time" value={toTimeInput(Number(daily[1]) || 0)} onChange={(e) => mtProps.onChange({ dailyTime: [daily[0] ?? 0, timeInputToTs(Number(daily[1]) || 0, e.target.value)] })} />
                                        </SettingRow>
                                    </>
                                )}
                            </>
                        )}
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picScale"
                            label="图片比例"
                            options={[
                                { value: "3:2", label: "3:2" },
                                { value: "1:1", label: "1:1" },
                                { value: "3:4", label: "3:4" },
                                { value: "16:9", label: "16:9" }
                            ]}
                        />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" label="图片填充" />
                        <RangeField data={data} onChange={onChange} field="productWidth" label="商品宽度" min={25} max={50} />
                        <RangeField data={data} onChange={onChange} field="productRadius" label="商品圆角" max={40} />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="自定义商品标题/卖点">
                        <RadioRowField data={data} onChange={onChange} field="showProdcutTitles" label="是否显示" options={SHOW_OPTIONS} />
                        {Number(data?.showProdcutTitles) === 1 ? (
                            <>
                                <RadioRowField data={data} onChange={onChange} field="showProductName" label="商品名称" options={SHOW_OPTIONS} />
                                {Number(data?.showProductName) === 1 ? (
                                    <>
                                        <RangeField data={data} onChange={onChange} field="productNameSize" label="商品名称大小" min={12} max={16} />
                                        <RadioRowField data={data} onChange={onChange} field="productNameWeight" label="商品名称粗细" options={WEIGHT_OPTIONS} />
                                    </>
                                ) : null}
                                <RadioRowField data={data} onChange={onChange} field="showSellingPoints" label="商品卖点" options={SHOW_OPTIONS} />
                            </>
                        ) : null}
                    </SettingGroup>
                    <SettingGroup title="商品价格设置">
                        <RadioRowField data={data} onChange={onChange} field="showProductPrice" label="商品价格" options={SHOW_OPTIONS} />
                        {Number(data?.showProductPrice) === 1 ? (
                            <RadioRowField data={data} onChange={onChange} field="priceWeight" label="商品价格粗细" options={WEIGHT_OPTIONS} />
                        ) : null}
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="颜色设置">
                        <ColorField {...mcProps} field="titleColor" label="模块大标题颜色" resetValue="" />
                        <ColorField {...mcProps} field="subTitleColor" label="模块小标题颜色" resetValue="" />
                        <ColorField {...mcProps} field="countdownColor" label="倒计时颜色" resetValue="" />
                        <ColorField {...mcProps} field="countdownTitleColor" label="倒计时文案颜色" resetValue="" />
                        {mt.topRightType === "button" ? (
                            <>
                                <SettingRow label="按钮渐变颜色">
                                    <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                                        <AlphaColorInput value={mc.buttonGradientColorA ?? ""} onChange={(v: string) => mcProps.onChange({ buttonGradientColorA: v })} />
                                        <span style={{ color: "#999" }}>-</span>
                                        <AlphaColorInput value={mc.buttonGradientColorB ?? ""} onChange={(v: string) => mcProps.onChange({ buttonGradientColorB: v })} />
                                    </span>
                                </SettingRow>
                                <div style={{ padding: "0 16px 10px", fontSize: 12, lineHeight: "18px", color: "#999" }}>提示：留空即跟随「模块颜色风格」自动变色</div>
                                <ColorField {...mcProps} field="buttonTextColor" label="按钮文字颜色" resetValue="#ffffff" />
                            </>
                        ) : null}
                    </SettingGroup>
                    <DecDivider />
                    <ColorField {...mcProps} field="productNameColor" label="商品名称颜色" resetValue="" />
                    <ColorField {...mcProps} field="sellingPointsColor" label="商品卖点颜色" resetValue="" />
                    <ExtensionStyleSettings data={data} onChange={onChange} type="specialOffer" />
                </>
            </SettingsTabs>
        </div>
    );
}
