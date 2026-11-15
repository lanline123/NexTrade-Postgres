/**
 * 新人二重礼 设置（对齐 view/admin mobilePro/newcomerCombo/Settings.vue）
 *
 * 页签：基础设置（头部装饰 / 1 重礼·商品设置 / 2 重礼·优惠券设置）/ 扩展设置
 * （商品配色 / 优惠券配色 / 整体修饰 / 容器设置；原版无 ModuleStyle/ContentStyle 页签）。
 * 查看更多链接 SelectLink 以文本输入承接，写回 section*MoreLink 同一结构（{link,title}）。
 * 优惠券原版经 DialogForm 选择真实优惠券（couponId + 接口补 info）；本编辑器无现成
 * 优惠券选择 API，按口径以本地列表编辑承接原数据结构：couponId/btnText/info
 * （券名/类型/金额或折扣/门槛/减免类型）逐项可编辑，Preview 直接消费内嵌 info。
 */
import { Input, InputNumber } from "@douyinfe/semi-ui";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RadioField,
    RangeField,
    ColorField,
    PicSelectField,
    PicLinkField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { newcomerComboDefaults as defaults } from "./index";
import ProductsSourceField from "../../../shared/ProductsSourceField";
import { ColorPairField } from "../../../../../components/ColorField";
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
const BUY_BTN_OPTIONS = [
    { value: 0, label: "不显示" },
    { value: 1, label: "样式一" },
    { value: 2, label: "样式二" },
    { value: 3, label: "样式三" },
    { value: 4, label: "样式四" },
    { value: 5, label: "样式五" },
    { value: 6, label: "样式六" },
    { value: 7, label: "样式七" },
    { value: 8, label: "样式八" }
];

function GradientPairField({ data, onChange, startField, endField, label }: Props & { startField: string; endField: string; label: string }) {
    return (
        <SettingRow label={label}>
            <ColorPairField
                value={String(data?.[startField] ?? "")}
                secondValue={String(data?.[endField] ?? "")}
                onChange={(v) => onChange({ [startField]: v })}
                onSecondChange={(v) => onChange({ [endField]: v })}
                onReset={() => onChange({ [startField]: (defaults as any)[startField] ?? "", [endField]: (defaults as any)[endField] ?? "" })}
            />
        </SettingRow>
    );
}

function BlockTextField({ data, onChange, field, label, placeholder, maxLength }: Props & { field: string; label: string; placeholder?: string; maxLength?: number }) {
    const { t } = useI18n();
    return (
        <SettingRow label={label} block>
            <Input
                value={data?.[field] ?? ""}
                placeholder={placeholder ? t(placeholder) : undefined}
                maxLength={maxLength}
                showClear
                style={{ width: "100%" }}
                onChange={(v: string) => onChange({ [field]: v })}
            />
        </SettingRow>
    );
}

function LinkTextField({ data, onChange, field, label }: Props & { field: string; label: string }) {
    // 字段为 { link, title } 对象（个别存量 link 再嵌一层，读取时拍平）：走共享 PicLinkField 选择，
    // 写回时保留对象其余键、link/title 更新为选择结果（嵌套存量在首次编辑时拍平为标准形状，渲染端两种形状都读）
    const linkObj = data?.[field] ?? {};
    const url = typeof linkObj.link === "string" ? linkObj.link : linkObj.link?.link ?? "";
    return (
        <SettingRow label={label} block>
            <PicLinkField
                value={{ link: url, title: typeof linkObj.title === "string" ? linkObj.title : "" }}
                onChange={(v) =>
                    onChange({
                        [field]:
                            linkObj && typeof linkObj === "object"
                                ? { ...linkObj, link: v.link, title: v.title || linkObj.title || "" }
                                : { link: v.link, title: v.title || "" }
                    })
                }
            />
        </SettingRow>
    );
}

function CouponListEditor({ data, onChange }: Props) {
    const { t } = useI18n();
    const coupons: Array<Record<string, any>> = Array.isArray(data?.coupons) ? data.coupons : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ coupons: next });
    const patchCoupon = (index: number, patch: Patch) => setList(coupons.map((c, i) => (i === index ? { ...c, ...patch } : c)));
    const patchInfo = (index: number, patch: Patch) =>
        patchCoupon(index, { info: { ...(coupons[index]?.info ?? {}), ...patch } });
    const addCoupon = () =>
        setList([
            ...coupons,
            {
                couponId: 0,
                btnText: "立即领取",
                info: { couponName: "新人专享券", couponType: 1, couponMoney: 10, couponDiscount: 9, minOrderAmount: 0, reduceType: 2 }
            }
        ]);
    return (
        <div style={{ padding: "0 16px 15px" }}>
            {coupons.map((c, index) => {
                const info = c.info ?? {};
                const infoProps: Props = { data: info, onChange: (patch: Patch) => patchInfo(index, patch) };
                return (
                    <div
                        key={index}
                        style={{ position: "relative", backgroundColor: "#fff", borderRadius: 2, boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)", padding: "10px 12px", marginTop: 12 }}
                    >
                        <div style={{ fontSize: 13, color: "#333", marginBottom: 6 }}>
                            优惠券: {info.couponName || "信息缺失"}
                        </div>
                        <BlockTextField {...infoProps} field="couponName" label="券名称" placeholder="如：新人专享 满600减40" />
                        <RadioRowField
                            {...infoProps}
                            field="couponType"
                            label="券类型"
                            options={[
                                { value: 1, label: "满减券" },
                                { value: 2, label: "折扣券" }
                            ]}
                        />
                        {Number(info.couponType) === 2 ? (
                            <SettingRow label="折扣">
                                <InputNumber value={Number(info.couponDiscount ?? 0)} min={0} max={10} step={0.1} style={{ width: 120 }} onChange={(v: any) => patchInfo(index, { couponDiscount: Number(v) || 0 })} />
                            </SettingRow>
                        ) : (
                            <SettingRow label="券金额">
                                <InputNumber value={Number(info.couponMoney ?? 0)} min={0} style={{ width: 120 }} onChange={(v: any) => patchInfo(index, { couponMoney: Number(v) || 0 })} />
                            </SettingRow>
                        )}
                        <RadioRowField
                            {...infoProps}
                            field="reduceType"
                            label="门槛类型"
                            options={[
                                { value: 1, label: "满金额可用" },
                                { value: 2, label: "无门槛" }
                            ]}
                        />
                        {Number(info.reduceType) !== 2 ? (
                            <SettingRow label="使用门槛">
                                <InputNumber value={Number(info.minOrderAmount ?? 0)} min={0} style={{ width: 120 }} onChange={(v: any) => patchInfo(index, { minOrderAmount: Number(v) || 0 })} />
                            </SettingRow>
                        ) : null}
                        <BlockTextField data={c} onChange={(patch: Patch) => patchCoupon(index, patch)} field="btnText" label="按钮文案" placeholder="立即领取" />
                        <span
                            title={t("删除")}
                            onClick={() => setList(coupons.filter((_, i) => i !== index))}
                            style={{ position: "absolute", right: -10, top: -10, color: "#fff", background: "#bbb", border: "2px solid #fff", borderRadius: "50%", zIndex: 2, width: 18, height: 18, textAlign: "center", lineHeight: "16px", cursor: "pointer", fontSize: 12, boxSizing: "border-box" }}
                        >
                            ×
                        </span>
                    </div>
                );
            })}
            <a className="dec-pic-add-btn" onClick={addCoupon} style={{ marginTop: 12 }}>
                {t("添加优惠券")}
            </a>
            <div style={{ color: "#999", fontSize: 12, lineHeight: 1.6, marginTop: 8 }}>
                说明：原版经弹窗选择真实优惠券并由接口回填券面信息；本编辑器以本地字段承接同一数据结构（couponId 由真实券选择产生，本地新增为 0）。
            </div>
        </div>
    );
}

export default function NewcomerComboSettings({ data, onChange }: Props) {
    const products = data?.products ?? {};
    const productCount =
        Number(products.productSelectType) === 1 ? (Array.isArray(products.productIds) ? products.productIds.length : 0) : Number(products.productNumber) || 0;
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

    const productColorRows: Array<{ field: string; label: string }> = [
        { field: "productTitleColor", label: "商品标题色" },
        { field: "productBriefColor", label: "描述文字色" },
        { field: "productPriceColor", label: "商品价格色" },
        { field: "productMarketPriceColor", label: "商品原价色" },
        { field: "productBriefTextColor", label: "卖点文字色" },
        { field: "buyBtnTextColor", label: "按钮文字色" }
    ];
    const couponColorRows: Array<{ field: string; label: string }> = [
        { field: "couponAmountColor", label: "金额文字色" },
        { field: "couponConditionColor", label: "条件文字色" },
        { field: "couponBtnTextColor", label: "按钮文字色" }
    ];
    const decorationColorRows: Array<{ field: string; label: string }> = [
        { field: "contentBgColor", label: "商品信息背景色" },
        { field: "decorationBgColor", label: "修饰背景色" }
    ];

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["基础设置", "扩展设置"]}>
                <>
                    <SettingGroup title="头部装饰">
                        <RadioRowField data={data} onChange={onChange} field="showHeader" label="显示头部" options={SHOW_OPTIONS} />
                        {Number(data?.showHeader) === 1 ? (
                            <>
                                <BlockTextField data={data} onChange={onChange} field="headerTitle" label="主标题" placeholder="如：新人专享二重礼" maxLength={20} />
                                <BlockTextField data={data} onChange={onChange} field="headerSubTitle" label="副标题" placeholder="如：限量供应 先到先得" maxLength={20} />
                            </>
                        ) : null}
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="1 重礼·商品设置">
                        <RadioRowField data={data} onChange={onChange} field="showSection1" label="显示本区块" options={SHOW_OPTIONS} />
                        {Number(data?.showSection1) === 1 ? (
                            <>
                                <BlockTextField data={data} onChange={onChange} field="section1Badge" label="徽章文字" placeholder="如：1重礼" maxLength={6} />
                                <BlockTextField data={data} onChange={onChange} field="section1Title" label="副标题" placeholder="如：新人尝鲜价" maxLength={12} />
                                <LinkTextField data={data} onChange={onChange} field="section1MoreLink" label="查看更多链接" />
                                <ProductsSourceField data={data} onChange={onChange} />
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
                                <RangeField data={data} onChange={onChange} field="productImgRadius" label="商品图片圆角" max={30} />
                                <RangeField data={data} onChange={onChange} field="productPercent" label="商品百分比" min={20} max={50} />
                                <RadioRowField data={data} onChange={onChange} field="showProductName" label="显示商品名称" options={SHOW_OPTIONS} />
                                <RangeField data={data} onChange={onChange} field="productItemGap" label="商品外边距" max={20} />
                                <RangeField data={data} onChange={onChange} field="productItemPadding" label="商品内边距" max={20} />
                                <RadioRowField data={data} onChange={onChange} field="showMarketPrice" label="显示划线价" options={SHOW_OPTIONS} />
                                <SettingRow label="购买按钮样式" block>
                                    <RadioField data={data} onChange={onChange} field="buyBtnStyle" options={BUY_BTN_OPTIONS} />
                                </SettingRow>
                            </>
                        ) : null}
                    </SettingGroup>
                    {Number(data?.showSection1) === 1 ? (
                        <SettingGroup title="商品卖点">
                            {productCount === 0 ? (
                                <div style={{ color: "#999", fontSize: 12, padding: "0 16px 10px", lineHeight: 1.6 }}>
                                    请先在上方「选择商品」中添加商品后再来配置卖点文字
                                </div>
                            ) : (
                                <div style={{ padding: "0 10px 10px" }}>
                                    {Array.from({ length: productCount }).map((_, index) => (
                                        <div key={index} style={{ backgroundColor: "#fff", borderRadius: 4, boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.15)", padding: "8px 10px 10px", marginTop: 10 }}>
                                            <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 13, fontWeight: 600, marginBottom: 6 }}>
                                                第 {index + 1} 个商品
                                                {isDynamic ? (
                                                    <span style={{ fontSize: 11, fontWeight: 400, color: "#999", padding: "1px 6px", backgroundColor: "#f5f7fa", borderRadius: 2 }}>动态拉取</span>
                                                ) : null}
                                            </div>
                                            <Input value={briefList[index] ?? ""} placeholder="如：新品 / 折扣" maxLength={6} showClear onChange={(v: string) => setBrief(index, v)} />
                                        </div>
                                    ))}
                                </div>
                            )}
                        </SettingGroup>
                    ) : null}
                    <DecDivider />
                    <SettingGroup title="2 重礼·优惠券设置">
                        <RadioRowField data={data} onChange={onChange} field="showSection2" label="显示本区块" options={SHOW_OPTIONS} />
                        {Number(data?.showSection2) === 1 ? (
                            <>
                                <BlockTextField data={data} onChange={onChange} field="section2Badge" label="徽章文字" placeholder="如：2重礼" maxLength={6} />
                                <BlockTextField data={data} onChange={onChange} field="section2Title" label="副标题" placeholder="如：新人专享礼包" maxLength={12} />
                                <LinkTextField data={data} onChange={onChange} field="section2MoreLink" label="查看更多链接" />
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="couponPerRow"
                                    label="优惠券布局"
                                    options={[
                                        { value: 1, label: "一行一个" },
                                        { value: 2, label: "一行两个" },
                                        { value: 3, label: "一行三个" },
                                        { value: 4, label: "一行四个" }
                                    ]}
                                />
                            </>
                        ) : null}
                    </SettingGroup>
                    {Number(data?.showSection2) === 1 ? (
                        <SettingGroup title="优惠券列表">
                            <CouponListEditor data={data} onChange={onChange} />
                        </SettingGroup>
                    ) : null}
                </>
                <>
                    <SettingGroup title="商品配色">
                        {productColorRows.map((row) => (
                            <ColorField key={row.field} data={data} onChange={onChange} field={row.field} label={row.label} resetValue={(defaults as any)[row.field] ?? ""} />
                        ))}
                        <GradientPairField data={data} onChange={onChange} startField="productBriefGradientStart" endField="productBriefGradientEnd" label="卖点背景渐变" />
                        <GradientPairField data={data} onChange={onChange} startField="buyBtnGradientStart" endField="buyBtnGradientEnd" label="按钮背景渐变" />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="优惠券配色">
                        <GradientPairField data={data} onChange={onChange} startField="couponBgGradientStart" endField="couponBgGradientEnd" label="优惠券背景渐变" />
                        {couponColorRows.map((row) => (
                            <ColorField key={row.field} data={data} onChange={onChange} field={row.field} label={row.label} resetValue={(defaults as any)[row.field] ?? ""} />
                        ))}
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="整体修饰">
                        {decorationColorRows.map((row) => (
                            <ColorField key={row.field} data={data} onChange={onChange} field={row.field} label={row.label} resetValue={(defaults as any)[row.field] ?? ""} />
                        ))}
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="容器设置">
                        <GradientPairField data={data} onChange={onChange} startField="boxGradientStart" endField="boxGradientEnd" label="外层背景渐变" />
                        <PicSelectField data={data} onChange={onChange} field="containerBgImage" label="容器背景图" />
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
