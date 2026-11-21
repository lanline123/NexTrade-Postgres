/**
 * 限时特惠 设置（对齐 view/admin mobilePro/limitedSpecial/Settings.vue）
 *
 * 页签：基础设置（商品设置 CommonProductEdit → ProductsSourceField / 商品卖点逐品
 * input maxlength 6 / 展示模式 / 显示内容）/ 扩展设置（商品配色 + 容器设置；
 * 原版无 ModuleStyle/ContentStyle 页签，故不接 ExtensionStyleSettings）。
 * 查看更多链接已恢复原版选链方式：PicLinkField（原版 SelectLink 绑定 module.moreLink
 * 整体，选中即回写完整 LinkType）。
 */
import { Input } from "@douyinfe/semi-ui";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RadioField,
    BuyBtnStyleField,
    RangeField,
    ColorField,
    PicSelectField,
    PicLinkField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { limitedSpecialDefaults as defaults } from "./index";
import ProductsSourceField from "../../../shared/ProductsSourceField";
import BuyBtnFlatFollowRow from "../../../shared/BuyBtnFlatFollowRow";
import { sanitizeObjStyle } from "../../../../../components/buyStyleObj";
import { usePcGlobalBuyStyle } from "../../../../decoratePc/previews/shared/PcBuyButton";
import { ColorPairField } from "../../../../../components/ColorField";
import { useI18n } from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

/** 渐变色对（共享 ColorPairField：重置链回默认值 + 两色 + 「-」分隔，与 PC 口径统一） */
function GradientPairField({
    data,
    onChange,
    startField,
    endField,
    label
}: Props & { startField: string; endField: string; label: string }) {
    const dStart = (defaults as any)[startField] ?? "";
    const dEnd = (defaults as any)[endField] ?? "";
    return (
        <SettingRow label={label}>
            <ColorPairField
                value={String(data?.[startField] ?? "")}
                secondValue={String(data?.[endField] ?? "")}
                onChange={(v) => onChange({ [startField]: v })}
                onSecondChange={(v) => onChange({ [endField]: v })}
                onReset={() => onChange({ [startField]: dStart, [endField]: dEnd })}
            />
        </SettingRow>
    );
}

const SHOW_OPTIONS = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

export default function LimitedSpecialSettings({ data, onChange }: Props) {
    const { t } = useI18n();
    const products = data?.products ?? {};
    const productCount =
        Number(products.productSelectType) === 1
            ? (Array.isArray(products.productIds) ? products.productIds.length : 0)
            : Number(products.productNumber) || 0;
    const isDynamic = Number(products.productSelectType) === 2 || Number(products.productSelectType) === 3;
    const briefList: string[] = Array.isArray(data?.productBriefList) ? data.productBriefList : [];
    const setBrief = (index: number, v: string) => {
        const next = [...briefList];
        while (next.length <= index) next.push("");
        next[index] = v;
        onChange({ productBriefList: next });
    };

    const frame = data?.frame ?? {};
    const frameProps: Props = { data: frame, onChange: (patch: Patch) => onChange({ frame: { ...frame, ...patch } }) };

    const moreLink = data?.moreLink ?? {};

    const colorRows: Array<{ field: string; label: string }> = [
        { field: "cardBgColor", label: "卡片背景色" },
        { field: "productTitleColor", label: "商品名称色" },
        { field: "productPriceColor", label: "商品价格色" },
        { field: "productMarketPriceColor", label: "商品原价色" },
        { field: "productSalesColor", label: "销量/评价色" },
        { field: "briefTextColor", label: "卖点文字色" },
        { field: "buyBtnTextColor", label: "按钮文字色" }
    ];

    // 扁平色族跟随（三期）：开启跟随后按钮渐变/文字色显示全局当前
    // 值并置灰只读（显示值取全局 objStyle 的 A/B 与 textColor；
    // 写回仍走原 onChange 锚定模块对象、从不落全局值）。
    const globalBuyStyle = usePcGlobalBuyStyle();
    const btnFollowing = Number(data?.buyBtnFollowGlobal) === 1;
    const globalObjStyle = sanitizeObjStyle(globalBuyStyle?.objStyle);
    const btnColorData = btnFollowing
        ? {
              ...data,
              buyBtnGradientStart: globalObjStyle.gradientColorA,
              buyBtnGradientEnd: globalObjStyle.gradientColorB,
              buyBtnTextColor: globalObjStyle.textColor
          }
        : data;
    const btnGreyed = btnFollowing ? { opacity: 0.55, pointerEvents: "none" as const } : undefined;

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["基础设置", "扩展设置"]}>
                <>
                    <SettingGroup title="商品设置">
                        <ProductsSourceField data={data} onChange={onChange} />
                    </SettingGroup>
                    <SettingGroup title="商品卖点">
                        <RadioRowField data={data} onChange={onChange} field="showBrief" label="显示卖点角标" options={SHOW_OPTIONS} />
                        {Number(data?.showBrief) === 1 ? (
                            productCount === 0 ? (
                                <div style={{ color: "#999", fontSize: 12, padding: "0 16px 10px", lineHeight: 1.6 }}>
                                    请先在上方「选择商品」中添加商品后再来配置卖点文字
                                </div>
                            ) : (
                                <div style={{ padding: "0 10px 10px" }}>
                                    {Array.from({ length: productCount }).map((_, index) => (
                                        <div
                                            key={index}
                                            style={{
                                                backgroundColor: "#fff",
                                                borderRadius: 4,
                                                boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.15)",
                                                padding: "8px 10px 10px",
                                                marginTop: 10
                                            }}
                                        >
                                            <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, fontWeight: 600, marginBottom: 6 }}>
                                                第 {index + 1} 个商品
                                                {isDynamic ? (
                                                    <span style={{ fontSize: 11, fontWeight: 400, color: "#999", padding: "1px 6px", backgroundColor: "#f5f7fa", borderRadius: 2 }}>
                                                        动态拉取
                                                    </span>
                                                ) : null}
                                            </div>
                                            <Input
                                                value={briefList[index] ?? ""}
                                                placeholder={t("如：新品 / 折扣")}
                                                maxLength={6}
                                                showClear
                                                onChange={(v: string) => setBrief(index, v)}
                                            />
                                        </div>
                                    ))}
                                </div>
                            )
                        ) : null}
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="展示模式">
                        <SettingRow label="商品图片比例" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="productPicRatio"
                                options={[
                                    { value: "default", label: "默认" },
                                    { value: "1:1", label: "1:1" },
                                    { value: "3:4", label: "3:4" },
                                    { value: "3:2", label: "3:2" }
                                ]}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="productPicFillType"
                            label="图片模式"
                            options={[
                                { value: "cover", label: "填充" },
                                { value: "contain", label: "周边留白" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="productImgRadius" label="图片圆角" max={30} />
                        <RangeField data={data} onChange={onChange} field="cardRadius" label="卡片圆角" max={30} />
                        <RangeField data={data} onChange={onChange} field="productPercent" label="商品百分比" min={20} max={50} />
                        <RangeField data={data} onChange={onChange} field="productItemGap" label="商品外边距" max={20} />
                        <RangeField data={data} onChange={onChange} field="productItemPadding" label="商品内边距" max={20} />
                        <SettingRow label="文字对齐" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="textAlign"
                                options={[
                                    { value: "left", label: "左对齐" },
                                    { value: "center", label: "居中" },
                                    { value: "right", label: "右对齐" }
                                ]}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="textWeight"
                            label="文字样式"
                            options={[
                                { value: 400, label: "常规体" },
                                { value: 700, label: "加粗体" }
                            ]}
                        />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="显示内容">
                        <RadioRowField data={data} onChange={onChange} field="showProductName" label="商品名称" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showPrice" label="商品价格" options={SHOW_OPTIONS} />
                        {Number(data?.showPrice) === 1 ? (
                            <>
                                <RangeField data={data} onChange={onChange} field="priceFontSize" label="价格字号" min={10} max={24} />
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="priceBold"
                                    label="价格粗体"
                                    options={[
                                        { value: 1, label: "粗体" },
                                        { value: 0, label: "常规" }
                                    ]}
                                />
                            </>
                        ) : null}
                        <RadioRowField data={data} onChange={onChange} field="showMarketPrice" label="划线原价" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showSales" label="商品销量" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showComment" label="商品评价" options={SHOW_OPTIONS} />
                        <SettingRow label="购买按钮样式" block>
                            <BuyBtnStyleField data={data} onChange={onChange} field="buyBtnStyle" />
                        </SettingRow>
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="商品配色">
                        {colorRows.map((row) =>
                            row.field === "buyBtnTextColor" ? (
                                <div key={row.field} style={btnGreyed}>
                                    <ColorField
                                        data={btnColorData}
                                        onChange={onChange}
                                        field={row.field}
                                        label={row.label}
                                        resetValue={(defaults as any)[row.field] ?? ""}
                                    />
                                </div>
                            ) : (
                                <ColorField
                                    key={row.field}
                                    data={data}
                                    onChange={onChange}
                                    field={row.field}
                                    label={row.label}
                                    resetValue={(defaults as any)[row.field] ?? ""}
                                />
                            )
                        )}
                        <GradientPairField data={data} onChange={onChange} startField="briefGradientStart" endField="briefGradientEnd" label="卖点背景渐变" />
                        <BuyBtnFlatFollowRow data={data} onChange={onChange} />
                        <div style={btnGreyed}>
                            <GradientPairField data={btnColorData} onChange={onChange} startField="buyBtnGradientStart" endField="buyBtnGradientEnd" label="按钮背景渐变" />
                        </div>
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="容器设置">
                        <PicSelectField data={data} onChange={onChange} field="containerBgImage" label="容器背景图" />
                        <GradientPairField data={data} onChange={onChange} startField="boxGradientStart" endField="boxGradientEnd" label="外层兜底渐变" />
                        <RangeField data={data} onChange={onChange} field="productAreaLeft" label="商品区左偏移" max={60} />
                        <div style={{ color: "#999", fontSize: 12, padding: "0 16px 10px", lineHeight: 1.6 }}>
                            左侧让出背景图文字区的宽度；该区域作为「查看更多」点击热区
                        </div>
                        <SettingRow label="查看更多链接" block>
                            <PicLinkField
                                value={typeof moreLink === "string" ? { link: moreLink, title: "" } : moreLink}
                                onChange={(v) => onChange({ moreLink: v })}
                            />
                        </SettingRow>
                        <RangeField data={data} onChange={onChange} field="productAreaPaddingY" label="商品区上下边距" max={30} />
                        <RangeField {...frameProps} field="boxRadius" label="圆角" max={30} />
                        <RangeField {...frameProps} field="boxPadding" label="左右边距" max={30} />
                        <RangeField {...frameProps} field="boxPaddingTop" label="上边距" max={30} />
                        <RangeField {...frameProps} field="boxPaddingBottom" label="下边距" max={30} />
                    </SettingGroup>
                </>
            </SettingsTabs>
        </div>
    );
}
