/**
 * 搜索结果 设置（对齐 view/admin mobileSearchResult/Settings.vue）
 *
 * 页签 内容设置/扩展设置（原版 SettingsTabs 的 content/extend 两槽）。
 * 内容（商品模式）：布局样式（带 value 文案）→ 列表样式（带 value）→ 商品标题颜色 →
 * 商品标题行数（带 value）→ 购物车按钮样式 0~8（仅双列，3 列换行分段组，带 value）→
 * 显示优惠券（双列为 el-switch 写 boolean / 单列为提示）→ 显示已售文字 / 显示店铺标签
 * （单列 switch / 双列提示）→ 显示添加购物车（仅单列整组渲染）。
 * 内容（门店模式）：门店信息 5 开关（radio 显示/不显示，写 0/1 数字）→ 门店名称
 * （颜色 / 字号滑杆 12~24 / 粗细 400/500/700 / Logo 圆角 max50）→ 门店商品
 * （名称/价格开关、名称行数、商品间距 max30）→ 布局间距（门店间距/上下/左右内间距/圆角，均 max30）。
 * 扩展：颜色组（模块背景色 / 搜索背景色 / 商品·门店背景色 / 主题色，主题色联动 priceColor 同步写回，
 * 同原版 onColorChange）+ 滑杆组（搜索项左右内间距 / 列表外边距 / 模块上下 / 模块左右内间距 /
 * 商品卡圆角，均 max50）。旧单一 padding 在新字段缺省时作回退显示值（同原版打开设置的继承逻辑）。
 *
 * 门控口径：原版 Settings 的 storeStyleAvailable 为写死 false 的 ref（快照内无赋值处），
 * 门店分支在原版实际不可达；types.ts 注释为「仅多门店版本可配」，此处以 contract/version
 * 的 isStore() 承接该意图：isStore() && searchResultType===2 时才进门店分支（同 storeNav 的映射法）。
 * 原版无 searchResultType 切换控件（审计所称该项与源码不符，见小结），故不新增。
 */
import { Switch } from "@douyinfe/semi-ui";
import {
    SettingRow,
    SegmentedRadio,
    BuyBtnStyleRadio,
    RadioRowField,
    RangeField,
    ColorField,
    DecDivider,
    DecSpreadTitle
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { isStore } from "../../../contract/version";
import { mobileSearchResultDefaults } from "./index";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

/** 历史草稿曾把 boolean/number 存成字符串，统一归一化后再比较（同原版 asBool） */
const asBool = (v: unknown, fallback = true): boolean => {
    if (v === undefined || v === null || v === "") return fallback;
    if (typeof v === "boolean") return v;
    if (typeof v === "number") return v !== 0;
    if (typeof v === "string") {
        const s = v.trim().toLowerCase();
        if (s === "true" || s === "1") return true;
        if (s === "false" || s === "0") return false;
    }
    return fallback;
};

/** 带 value 文案的单选行（原版 dec-edit-group-title 内 label + value 上下两行） */
function RadioWithValue({
    label,
    valueText,
    value,
    options,
    onPick
}: {
    label: string;
    valueText: string;
    value: any;
    options: Array<{ value: number | string; label: string }>;
    onPick: (v: number | string) => void;
}) {
    return (
        <div className="dec-edit-group">
            <div className="dec-edit-group-title">
                <div className="label">{label}</div>
                <div className="value" style={{ fontSize: 14, fontWeight: 500, color: "#323233" }}>
                    {valueText}
                </div>
            </div>
            <div className="dec-edit-group-con">
                <SegmentedRadio value={value} options={options} onChange={onPick} />
            </div>
        </div>
    );
}

/** boolean 开关行：可用时 Semi Switch 写 boolean，不可用时原版灰字提示（.dec-edit-tip） */
function BoolSwitchOrTip({
    label,
    available,
    tip,
    checked,
    onPick
}: {
    label: string;
    available: boolean;
    tip: string;
    checked: boolean;
    onPick: (v: boolean) => void;
}) {
    return (
        <SettingRow label={label}>
            {available ? (
                <Switch checked={checked} onChange={(v: boolean) => onPick(!!v)} />
            ) : (
                <span style={{ color: "#999", fontSize: 12 }}>{tip}</span>
            )}
        </SettingRow>
    );
}

const STORE_INFO_FIELDS: Array<{ field: string; label: string }> = [
    { field: "storeShowRating", label: "显示门店评分" },
    { field: "storeShowMinOrder", label: "显示起送价" },
    { field: "storeShowDeliveryFee", label: "显示配送费" },
    { field: "storeShowDistance", label: "显示距离" },
    { field: "storeShowFollow", label: "显示关注按钮" }
];
const STORE_PRODUCT_FIELDS: Array<{ field: string; label: string }> = [
    { field: "storeShowProductName", label: "显示门店商品名称" },
    { field: "storeShowProductPrice", label: "显示门店商品价格" }
];
const BUY_BTN_LABELS = ["不显示", "样式一", "样式二", "样式三", "样式四", "样式五", "样式六", "样式七", "样式八"];

export default function MobileSearchResultSettings({ data, onChange }: Props) {
    const searchResultType = Number(data?.searchResultType ?? 1);
    const isStoreResult = isStore() && searchResultType === 2;
    const layoutStyle = Number(data?.layoutStyle ?? 1);
    const listStyle = Number(data?.listStyle ?? 1);
    const productTitleLine = Number(data?.productTitleLine ?? 2);
    const buyBtnStyle = Number(data?.buyBtnStyle ?? 0);
    const storeProductNameRow = Number(data?.storeProductNameRow ?? 1);
    const showStoreProductName = asBool(data?.storeShowProductName, true);

    // 门店开关在原版以 asBool 归一化后按 1/0 呈现/写回（字段契约本身为 0/1 数字）
    const storeData: Record<string, any> = { ...data };
    [...STORE_INFO_FIELDS, ...STORE_PRODUCT_FIELDS].forEach(({ field }) => {
        storeData[field] = asBool(data?.[field], true) ? 1 : 0;
    });

    // 主题色联动价格颜色（一次性 patch 写回，同原版 updateFields，避免后一次写入被丢）
    const themeOnChange = (patch: Patch) => {
        if (patch.themeColor !== undefined) {
            onChange({ ...patch, priceColor: patch.themeColor });
            return;
        }
        onChange(patch);
    };

    // 扩展滑杆：旧单一 padding 在新字段缺省时作回退（同原版打开设置时的继承）
    const rangeData: Record<string, any> = {
        ...data,
        paddingTopBottom: data?.paddingTopBottom ?? data?.padding ?? mobileSearchResultDefaults.paddingTopBottom,
        paddingLeftRight: data?.paddingLeftRight ?? data?.padding ?? mobileSearchResultDefaults.paddingLeftRight
    };

    const colorFields: Array<{ field: string; label: string; resetValue: string }> = [
        { field: "backgroundColor", label: "模块背景色", resetValue: mobileSearchResultDefaults.backgroundColor },
        { field: "searchItemBackgroundColor", label: "搜索背景色", resetValue: mobileSearchResultDefaults.searchItemBackgroundColor },
        { field: "itemBackgroundColor", label: isStoreResult ? "门店背景色" : "商品背景色", resetValue: mobileSearchResultDefaults.itemBackgroundColor },
        { field: "themeColor", label: "主题色（价格、按钮、优惠券）", resetValue: mobileSearchResultDefaults.themeColor }
    ];
    const rangeFields: Array<{ field: string; label: string; max: number }> = [
        { field: "searchItemPaddingLeftRight", label: "搜索项左右内间距", max: 50 },
        { field: "margin", label: isStoreResult ? "门店列表外边距" : "商品列表外边距", max: 50 },
        { field: "paddingTopBottom", label: "模块上下内间距", max: 50 },
        { field: "paddingLeftRight", label: "模块左右内间距", max: 50 },
        { field: "productImageRadius", label: "商品卡圆角", max: 50 }
    ];

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["内容设置", "扩展设置"]}>
                <div>
                    {isStoreResult ? (
                        <>
                            <DecDivider />
                            <DecSpreadTitle title="门店信息" />
                            {STORE_INFO_FIELDS.map((item) => (
                                <RadioRowField
                                    key={item.field}
                                    data={storeData}
                                    onChange={onChange}
                                    field={item.field}
                                    label={item.label}
                                    options={[
                                        { value: 1, label: "显示" },
                                        { value: 0, label: "不显示" }
                                    ]}
                                />
                            ))}

                            <DecDivider />
                            <DecSpreadTitle title="门店名称" />
                            <ColorField
                                data={data}
                                onChange={onChange}
                                field="storeShopTitleColor"
                                label="门店名称颜色"
                                resetValue={mobileSearchResultDefaults.storeShopTitleColor}
                            />
                            <RangeField data={data} onChange={onChange} field="storeShopTitleFontSize" label="门店名称字号" min={12} max={24} />
                            <RadioRowField
                                data={data}
                                onChange={(patch) => onChange({ storeShopTitleFontBold: Number(patch.storeShopTitleFontBold) })}
                                field="storeShopTitleFontBold"
                                label="门店名称粗细"
                                options={[
                                    { value: 400, label: "常规" },
                                    { value: 500, label: "中粗" },
                                    { value: 700, label: "加粗" }
                                ]}
                            />
                            <RangeField data={data} onChange={onChange} field="storeLogoRadius" label="门店Logo圆角" max={50} />

                            <DecDivider />
                            <DecSpreadTitle title="门店商品" />
                            {STORE_PRODUCT_FIELDS.map((item) => (
                                <RadioRowField
                                    key={item.field}
                                    data={storeData}
                                    onChange={onChange}
                                    field={item.field}
                                    label={item.label}
                                    options={[
                                        { value: 1, label: "显示" },
                                        { value: 0, label: "不显示" }
                                    ]}
                                />
                            ))}
                            {showStoreProductName ? (
                                <RadioRowField
                                    data={{ storeProductNameRow }}
                                    onChange={(patch) => onChange({ storeProductNameRow: Number(patch.storeProductNameRow) })}
                                    field="storeProductNameRow"
                                    label="门店商品名称行数"
                                    options={[
                                        { value: 1, label: "1行" },
                                        { value: 2, label: "2行" }
                                    ]}
                                />
                            ) : null}
                            <RangeField data={data} onChange={onChange} field="storeProductGap" label="门店商品间距" max={30} />

                            <DecDivider />
                            <DecSpreadTitle title="布局间距" />
                            <RangeField data={data} onChange={onChange} field="storeGap" label="门店间距" max={30} />
                            <RangeField data={data} onChange={onChange} field="storePaddingTopBottom" label="门店上下内间距" max={30} />
                            <RangeField data={data} onChange={onChange} field="storePaddingLeftRight" label="门店左右内间距" max={30} />
                            <RangeField data={data} onChange={onChange} field="storeCardRadius" label="门店圆角" max={30} />
                        </>
                    ) : (
                        <>
                            <RadioWithValue
                                label="布局样式"
                                valueText={layoutStyle === 1 ? "国内版" : "海外版"}
                                value={layoutStyle}
                                options={[
                                    { value: 1, label: "国内版" },
                                    { value: 2, label: "海外版" }
                                ]}
                                onPick={(v) => onChange({ layoutStyle: Number(v) })}
                            />
                            <RadioWithValue
                                label="列表样式"
                                valueText={listStyle === 1 ? "双列网格" : "单列列表"}
                                value={listStyle}
                                options={[
                                    { value: 1, label: "双列网格" },
                                    { value: 2, label: "单列列表" }
                                ]}
                                onPick={(v) => onChange({ listStyle: Number(v) })}
                            />
                            <ColorField
                                data={data}
                                onChange={onChange}
                                field="productTitleColor"
                                label="商品标题颜色"
                                resetValue={mobileSearchResultDefaults.productTitleColor}
                            />
                            <RadioWithValue
                                label="商品标题行数"
                                valueText={productTitleLine === 1 ? "1行" : "2行"}
                                value={productTitleLine}
                                options={[
                                    { value: 1, label: "1行" },
                                    { value: 2, label: "2行" }
                                ]}
                                onPick={(v) => onChange({ productTitleLine: Number(v) })}
                            />
                            {listStyle === 1 ? (
                                <div className="dec-edit-group dec-edit-group-block">
                                    <div className="dec-edit-group-title">
                                        <div className="label">购物车按钮样式（双列）</div>
                                        <div className="value" style={{ fontSize: 14, fontWeight: 500, color: "#323233" }}>
                                            {BUY_BTN_LABELS[buyBtnStyle] ?? "不显示"}
                                        </div>
                                    </div>
                                    <div className="dec-edit-group-con">
                                        {/* 2026-11-21 改可视化点选（共享 BuyBtnStylePicker）；原版为 .buy-btn-style-group 文字钮 3 列换行 */}
                                        <BuyBtnStyleRadio value={buyBtnStyle} onChange={(v) => onChange({ buyBtnStyle: v })} />
                                    </div>
                                </div>
                            ) : null}
                            <BoolSwitchOrTip
                                label="显示优惠券（双列）"
                                available={listStyle === 1}
                                tip="单列模式不显示优惠券"
                                checked={asBool(data?.showCoupon, true)}
                                onPick={(v) => onChange({ showCoupon: v })}
                            />
                            <BoolSwitchOrTip
                                label="显示已售文字"
                                available={listStyle === 2}
                                tip="双列模式不显示销量"
                                checked={asBool(data?.showSales, true)}
                                onPick={(v) => onChange({ showSales: v })}
                            />
                            <BoolSwitchOrTip
                                label="显示店铺标签"
                                available={listStyle === 2}
                                tip="双列模式不显示店铺标签"
                                checked={asBool(data?.showShopTag, true)}
                                onPick={(v) => onChange({ showShopTag: v })}
                            />
                            {listStyle === 2 ? (
                                <SettingRow label="显示添加购物车（单列）">
                                    <Switch checked={asBool(data?.showCartButton, true)} onChange={(v: boolean) => onChange({ showCartButton: !!v })} />
                                </SettingRow>
                            ) : null}
                        </>
                    )}
                </div>
                <div>
                    {colorFields.map((item) =>
                        item.field === "themeColor" ? (
                            <ColorField
                                key={item.field}
                                data={data}
                                onChange={themeOnChange}
                                field={item.field}
                                label={item.label}
                                resetValue={item.resetValue}
                            />
                        ) : (
                            <ColorField key={item.field} data={data} onChange={onChange} field={item.field} label={item.label} resetValue={item.resetValue} />
                        )
                    )}
                    {rangeFields.map((item) => (
                        <RangeField key={item.field} data={rangeData} onChange={onChange} field={item.field} label={item.label} max={item.max} />
                    ))}
                </div>
            </SettingsTabs>
        </div>
    );
}
