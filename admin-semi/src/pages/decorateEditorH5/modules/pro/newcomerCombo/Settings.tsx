/**
 * 新人二重礼 设置（对齐 view/admin mobilePro/newcomerCombo/Settings.vue）
 *
 * 页签：基础设置（头部装饰 / 1 重礼·商品设置 / 2 重礼·优惠券设置）/ 扩展设置
 * （商品配色 / 优惠券配色 / 整体修饰 / 容器设置；原版无 ModuleStyle/ContentStyle 页签）。
 * 查看更多链接 SelectLink 以文本输入承接，写回 section*MoreLink 同一结构（{link,title}）。
 * 优惠券对齐原版数据模型：条目只存 { couponId, btnText }，券面 info 不落库，
 * 由 useCouponInfoMap 按 couponId 从接口拉取（Settings 列表名、Preview 券面同源）；
 * 「添加优惠券」打开多选弹窗（语义同原版 MultiCouponSelectList：返回最终选中的完整列表，
 * 取消勾选即移除，已存在券保留原 btnText、新券默认「立即领取」）；列表可拖拽排序、hover 删除，
 * 条目无字段编辑入口（同原版）。存量旧数据（couponId=0 + 内嵌 info）列表名以其内嵌 info 兜底展示，
 * 经弹窗重新确认后即按原版模型改写为 { couponId, btnText }。
 */
import { useEffect, useRef, useState } from "react";
import { Checkbox, Input, Modal, Spin } from "@douyinfe/semi-ui";
import { apiGet } from "../../../../../api";
import { useCouponInfoMap } from "../../../shared/useCouponInfoMap";
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
    const { couponInfoMap, fetchCouponInfo, removeCouponInfo } = useCouponInfoMap();
    const [infoLoading, setInfoLoading] = useState(false);
    const [pickerOpen, setPickerOpen] = useState(false);
    const [pickerLoading, setPickerLoading] = useState(false);
    const [pickerRows, setPickerRows] = useState<Array<Record<string, any>>>([]);
    const [pickerKeyword, setPickerKeyword] = useState("");
    const [checkedIds, setCheckedIds] = useState<number[]>([]);
    const dragIndexRef = useRef<number | null>(null);
    const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);

    // 与原版一致：couponId 集合变化即拉取券面 info（immediate，含首次打开）
    const idsKey = coupons
        .map((c) => Number(c.couponId) || 0)
        .filter((id) => id > 0)
        .join(",");
    useEffect(() => {
        if (!idsKey) return;
        let alive = true;
        setInfoLoading(true);
        fetchCouponInfo(idsKey.split(",").map(Number)).finally(() => {
            if (alive) setInfoLoading(false);
        });
        return () => {
            alive = false;
        };
    }, [idsKey, fetchCouponInfo]);

    // 展示用 info：接口 map 优先；存量内嵌 info（旧编辑器写入）仅作显示兜底，不写回
    const displayInfo = (c: Record<string, any>) => couponInfoMap.get(Number(c.couponId)) || c.info || null;

    const removeCoupon = (index: number) => {
        const removed = coupons[index];
        setList(coupons.filter((_, i) => i !== index));
        if (removed?.couponId) removeCouponInfo(Number(removed.couponId));
    };

    const moveCoupon = (from: number, to: number) => {
        if (from === to || from < 0 || to < 0 || from >= coupons.length || to >= coupons.length) return;
        const next = [...coupons];
        const [item] = next.splice(from, 1);
        next.splice(to, 0, item);
        setList(next);
    };

    const openPicker = () => {
        setCheckedIds(coupons.map((c) => Number(c.couponId) || 0).filter((id) => id > 0));
        setPickerKeyword("");
        setPickerOpen(true);
    };

    useEffect(() => {
        if (!pickerOpen) return;
        let alive = true;
        setPickerLoading(true);
        apiGet<any>("/promotion/coupon/list", { page: 1, size: 100, keyword: pickerKeyword || undefined })
            .then((res: any) => {
                if (alive) setPickerRows(res?.records || res?.list || []);
            })
            .catch(() => {
                if (alive) setPickerRows([]);
            })
            .finally(() => {
                if (alive) setPickerLoading(false);
            });
        return () => {
            alive = false;
        };
    }, [pickerOpen, pickerKeyword]);

    // 弹窗确认 = 最终选中的完整列表（同原版 onCouponsSelected）：保留已存在券的 btnText，新券默认「立即领取」
    const confirmPicker = () => {
        const existingBtnText = new Map(coupons.map((c) => [Number(c.couponId), c.btnText]));
        const kept = coupons
            .filter((c) => checkedIds.includes(Number(c.couponId)))
            .map((c) => ({ couponId: Number(c.couponId), btnText: existingBtnText.get(Number(c.couponId)) || "立即领取" }));
        const keptIds = new Set(kept.map((c) => c.couponId));
        const added = pickerRows
            .filter((r) => checkedIds.includes(Number(r.couponId)) && !keptIds.has(Number(r.couponId)))
            .map((r) => ({ couponId: Number(r.couponId), btnText: "立即领取" }));
        setList([...kept, ...added]);
        setPickerOpen(false);
    };

    const pickerFace = (r: Record<string, any>) =>
        Number(r.couponType) === 2 ? `${r.couponDiscount ?? "-"}折` : `¥${r.couponMoney ?? "-"}${Number(r.minOrderAmount) > 0 ? `（满${r.minOrderAmount}可用）` : "（无门槛）"}`;

    return (
        <div style={{ padding: "0 16px 15px" }}>
            {coupons.map((c, index) => {
                const info = displayInfo(c);
                return (
                    <div
                        key={`${Number(c.couponId) || 0}-${index}`}
                        onDragOver={(e) => {
                            e.preventDefault();
                            setDragOverIndex(index);
                        }}
                        onDrop={(e) => {
                            e.preventDefault();
                            if (dragIndexRef.current !== null) moveCoupon(dragIndexRef.current, index);
                            dragIndexRef.current = null;
                            setDragOverIndex(null);
                        }}
                        style={{
                            position: "relative",
                            display: "flex",
                            alignItems: "center",
                            gap: 8,
                            backgroundColor: "#fff",
                            borderRadius: 2,
                            boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)",
                            padding: "10px 12px",
                            marginTop: 12,
                            outline: dragOverIndex === index && dragIndexRef.current !== null && dragIndexRef.current !== index ? "1px dashed #155bd4" : undefined
                        }}
                    >
                        <span
                            draggable
                            onDragStart={() => {
                                dragIndexRef.current = index;
                            }}
                            onDragEnd={() => {
                                dragIndexRef.current = null;
                                setDragOverIndex(null);
                            }}
                            title={t("拖拽排序")}
                            style={{ flexShrink: 0, cursor: "move", display: "block", lineHeight: 0 }}
                        >
                            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
                                <line x1="2" y1="3.5" x2="12" y2="3.5" stroke="#bbb" strokeWidth="1.5" />
                                <line x1="2" y1="7" x2="12" y2="7" stroke="#bbb" strokeWidth="1.5" />
                                <line x1="2" y1="10.5" x2="12" y2="10.5" stroke="#bbb" strokeWidth="1.5" />
                            </svg>
                        </span>
                        <span style={{ flex: 1, fontSize: 13, color: "#333", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                            {t("优惠券")}: {info?.couponName || (infoLoading ? t("加载中...") : t("信息缺失"))}
                        </span>
                        <span
                            title={t("删除")}
                            onClick={() => removeCoupon(index)}
                            style={{ position: "absolute", right: -10, top: -10, color: "#fff", background: "#bbb", border: "2px solid #fff", borderRadius: "50%", zIndex: 2, width: 18, height: 18, textAlign: "center", lineHeight: "16px", cursor: "pointer", fontSize: 12, boxSizing: "border-box" }}
                        >
                            ×
                        </span>
                    </div>
                );
            })}
            <a className="dec-pic-add-btn" onClick={openPicker} style={{ marginTop: 12 }}>
                {t("添加优惠券")}
            </a>
            <Modal title={t("选择优惠券")} visible={pickerOpen} onCancel={() => setPickerOpen(false)} onOk={confirmPicker} okText={t("确定")} cancelText={t("取消")} width={700}>
                <Input
                    value={pickerKeyword}
                    placeholder={t("搜索优惠券名称")}
                    showClear
                    style={{ width: "100%", marginBottom: 12 }}
                    onChange={(v: string) => setPickerKeyword(v)}
                />
                <Spin spinning={pickerLoading}>
                    <div style={{ maxHeight: 420, overflowY: "auto", display: "flex", flexDirection: "column", gap: 6 }}>
                        {pickerRows.map((r) => {
                            const id = Number(r.couponId);
                            return (
                                <label key={id} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 10px", background: "#f7f8fa", borderRadius: 4, cursor: "pointer" }}>
                                    <Checkbox
                                        checked={checkedIds.includes(id)}
                                        onChange={(e: any) => setCheckedIds((prev) => (e?.target?.checked ? [...prev, id] : prev.filter((x) => x !== id)))}
                                    />
                                    <span style={{ flex: 1, fontSize: 13, color: "#333" }}>{r.couponName || "-"}</span>
                                    <span style={{ fontSize: 12, color: "#999" }}>{pickerFace(r)}</span>
                                </label>
                            );
                        })}
                        {!pickerLoading && !pickerRows.length ? (
                            <div style={{ color: "#999", fontSize: 12, padding: 12 }}>{t("暂无优惠券")}</div>
                        ) : null}
                    </div>
                </Spin>
            </Modal>
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
