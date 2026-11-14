/**
 * 共享样式编辑组件（移植自 view/admin editor/src/modules/src 的
 * moduleStyle/Edit.vue、contentStyle/Edit.vue、animation/Edit.vue、
 * commonSubTitle/Edit.vue、commonImageFill/Edit.vue）。
 *
 * - 字段门控与数值上限逐条按原版 type 条件复刻；数据键为 moduleStyle /
 *   contentStyle / animation / subTitle（与原版契约同名，默认值见
 *   contract/sharedDefaults 的 defaultModuleStyle 等）。
 * - 嵌套对象通过 onChange(patch) 浅合并协议写回，保存路径不变。
 * - 原版单选里的 iconfont 图标钮（图片填充/字号/粗细）在本实现以等值文字
 *   钮承接（分段组形态与值不变）。
 */
import { useState } from "react";
import AlphaColorInput from "../../../components/AlphaColorInput";
import {
    SettingRow,
    SegmentedRadio,
    RangeField,
    ColorField,
    PicSelectField,
    DecDivider,
    DecSpreadTitle
} from "./settingWidgets";
import { defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../contract/sharedDefaults";

import { useI18n } from '../../../i18n';
type Patch = Record<string, any>;
interface NestedProps {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

/** 嵌套对象适配：把顶层 data/onChange 转成某个子对象的 data/onChange */
function nested(data: Record<string, any>, onChange: (patch: Patch) => void, key: string) {
    const sub = (data?.[key] ?? {}) as Record<string, any>;
    return {
        data: sub,
        onChange: (patch: Patch) => onChange({ [key]: { ...sub, ...patch } })
    };
}

// ---------------- 页签（原版 Settings 内 el-tabs / Tabs 组件） ----------------

export function SettingsTabs({ tabs, children }: { tabs: string[]; children: React.ReactNode[] }) {
    const { t } = useI18n();
    const [active, setActive] = useState(0);
    return (
        <div className="h5-settings-tabs">
            <div
                className="h5-settings-tabs-head"
                style={{ display: "flex", borderBottom: "1px solid #ebeef5", margin: "0 -16px 6px", padding: "0 16px" }}
            >
                {tabs.map((label, i) => (
                    <div
                        key={label}
                        onClick={() => setActive(i)}
                        style={{
                            padding: "0 14px",
                            height: 40,
                            lineHeight: "40px",
                            fontSize: 14,
                            cursor: "pointer",
                            color: active === i ? "#155bd4" : "#969799",
                            fontWeight: active === i ? 600 : 400,
                            borderBottom: active === i ? "2px solid #155bd4" : "2px solid transparent",
                            marginBottom: -1
                        }}
                    >
                        {t(label)}
                    </div>
                ))}
            </div>
            <div className="h5-settings-tabs-panel">{children[active] ?? null}</div>
        </div>
    );
}

// ---------------- 图片填充（commonImageFill/Edit.vue） ----------------

export function PicFillTypeField({
    data,
    onChange,
    field,
    label = "图片填充"
}: NestedProps & { field: string; label?: string }) {
    return (
        <SettingRow label={label}>
            <SegmentedRadio
                value={data?.[field] ?? "cover"}
                options={[
                    { value: "cover", label: "填充" },
                    { value: "contain", label: "周边留白" }
                ]}
                onChange={(v) => onChange({ [field]: v })}
            />
        </SettingRow>
    );
}

// ---------------- 模块样式（moduleStyle/Edit.vue） ----------------

const MODULE_STYLE_SIDE_TYPES = [
    "categoryA1",
    "noticeA2",
    "noticeA3",
    "specialOffer",
    "mixedProductDisplay",
    "categoryRec",
    "productGrouping",
    "customProduct",
    "shopTips"
];
const MODULE_STYLE_NO_PIC_TYPES = ["article", "banner", "ComboSuggestionsEdit", "noticeA2", "mixedProductDisplay", "storeNav"];

export function ModuleStyleSettingsField({
    data,
    onChange,
    type = "",
    topPaddingMax = 30
}: NestedProps & { type?: string; topPaddingMax?: number }) {
    const ms = nested(data, onChange, "moduleStyle");
    return (
        <>
            {type !== "storeNav" ? (
                <>
                    <DecDivider />
                    <DecSpreadTitle title="模块样式设置" />
                </>
            ) : null}
            {type === "noticeA2" ? (
                <RangeField {...ms} field="boxPaddingTopBottom" label="模块上下间距" max={40} />
            ) : (
                <>
                    <RangeField {...ms} field="boxPaddingTop" label="模块顶部间距" max={topPaddingMax} />
                    <RangeField {...ms} field="boxPaddingBottom" label="模块底部间距" max={30} />
                </>
            )}
            {MODULE_STYLE_SIDE_TYPES.includes(type) ? (
                <RangeField {...ms} field="boxPadding" label="模块左右间距" max={30} />
            ) : null}
            {type === "shopTips" ? <RangeField {...ms} field="boxMargin" label="模块左右外间距" max={30} /> : null}
            {type === "banner" ? <RangeField {...ms} field="bannerMarginTop" label="模块顶部内间距" max={30} /> : null}
            <ColorField
                {...ms}
                field="backgroundColor"
                label="模块背景颜色"
                resetValue={defaultModuleStyle.backgroundColor}
            />
            {!MODULE_STYLE_NO_PIC_TYPES.includes(type) ? (
                <>
                    <PicSelectField {...ms} field="backgroundPic" label="模块背景图片" />
                    <PicFillTypeField {...ms} field="backgroundPicFillType" />
                </>
            ) : null}
        </>
    );
}

// ---------------- 内容区样式（contentStyle/Edit.vue） ----------------

const CONTENT_STYLE_INNER_PADDING_TYPES = ["imageTextComponent", "categoryA1", "mixedProductDisplay", "productGrouping"];
const CONTENT_STYLE_SINGLE_RADIUS_TYPES = [
    "imageTextComponent",
    "article",
    "noticeA2",
    "noticeA3",
    "specialOffer",
    "mixedProductDisplay",
    "countdown",
    "customProduct"
];
const CONTENT_STYLE_NO_GRADIENT_TYPES = ["imageTextComponent", "noticeA2", "noticeA3", "customProduct"];
const CONTENT_STYLE_BG_COLOR_TYPES = ["imageTextComponent", "noticeA2", "noticeA3", "customProduct"];
const CONTENT_STYLE_NO_PIC_TYPES = [
    "imageTextComponent",
    "article",
    "noticeA2",
    "noticeA3",
    "specialOffer",
    "countdown",
    "productGrouping",
    "customProduct"
];
const CONTENT_STYLE_NO_SHADOW_TYPES = ["imageTextComponent", "article", "productGrouping"];

export function ContentStyleSettingsField({
    data,
    onChange,
    type = "",
    showPaddingTop = false,
    paddingTopMax = 40
}: NestedProps & { type?: string; showPaddingTop?: boolean; paddingTopMax?: number }) {
    const { t } = useI18n();
    const cs = nested(data, onChange, "contentStyle");
    const sub = cs.data;
    const gradientOptions = [
        { value: "upDown", label: "上下" },
        { value: "leftRight", label: "左右" },
        { value: "diagonal", label: "斜向" },
        ...(type === "slideLargeImage" ? [{ value: "purity", label: "纯色" }] : [])
    ];
    return (
        <>
            <DecDivider />
            <DecSpreadTitle title="内容区样式设置" />
            {type === "noticeA2" || type === "noticeA3" ? (
                <RangeField {...cs} field="paddingTopBottom" label="内容区上下间距" max={40} />
            ) : null}
            {type === "categoryA2" || showPaddingTop ? (
                <RangeField {...cs} field="paddingTop" label="内容区上间距" max={paddingTopMax} />
            ) : null}
            {type === "categoryA2" ? <RangeField {...cs} field="paddingBottom" label="内容区下间距" max={40} /> : null}
            {type === "categoryA2" || type === "noticeA2" || type === "noticeA3" ? (
                <RangeField {...cs} field="paddingLeftRight" label="内容区左右间距" max={20} />
            ) : null}
            {CONTENT_STYLE_INNER_PADDING_TYPES.includes(type) ? (
                <RangeField {...cs} field="padding" label="内容区内部间距" max={40} />
            ) : null}
            {!CONTENT_STYLE_SINGLE_RADIUS_TYPES.includes(type) ? (
                <>
                    <RangeField {...cs} field="boxRadiusTop" label="内容区上部圆角" max={40} />
                    <RangeField {...cs} field="boxRadiusBottom" label="内容区下部圆角" max={40} />
                </>
            ) : (
                <RangeField {...cs} field="boxRadius" label="内容区圆角半径" max={40} />
            )}
            {!CONTENT_STYLE_NO_GRADIENT_TYPES.includes(type) ? (
                <>
                    <SettingRow label="背景渐变方向">
                        <SegmentedRadio
                            value={sub.gradientType ?? "upDown"}
                            options={gradientOptions}
                            onChange={(v) => cs.onChange({ gradientType: v })}
                        />
                    </SettingRow>
                    <SettingRow label="背景渐变颜色">
                        <div className="dec-color-group">
                            <div className="dec-color-button">
                                <a
                                    className="dec-color-reset"
                                    onClick={() =>
                                        cs.onChange({
                                            gradientColorA: defaultContentStyle.gradientColorA,
                                            gradientColorB: defaultContentStyle.gradientColorB
                                        })
                                    }
                                >
                                    {t('重置')}
                                </a>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput
                                        value={String(sub.gradientColorA ?? "")}
                                        onChange={(v) => cs.onChange({ gradientColorA: v })}
                                    />
                                </span>
                                <span style={{ margin: "0 6px", color: "#969799" }}>-</span>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput
                                        value={String(sub.gradientColorB ?? "")}
                                        onChange={(v) => cs.onChange({ gradientColorB: v })}
                                    />
                                </span>
                            </div>
                        </div>
                    </SettingRow>
                </>
            ) : null}
            {CONTENT_STYLE_BG_COLOR_TYPES.includes(type) ? (
                <ColorField {...cs} field="backgroundColor" label="内容区背景颜色" resetValue={defaultContentStyle.backgroundColor} />
            ) : null}
            {!CONTENT_STYLE_NO_PIC_TYPES.includes(type) ? (
                <>
                    <PicSelectField {...cs} field="backgroundPic" label="内容区背景图片" />
                    <PicFillTypeField {...cs} field="backgroundPicFillType" label="背景图片填充" />
                </>
            ) : null}
            {!CONTENT_STYLE_NO_SHADOW_TYPES.includes(type) ? (
                <SettingRow label="内容区投影">
                    <SegmentedRadio
                        value={Number(sub.boxShadow ?? 0)}
                        options={[
                            { value: 1, label: "显示" },
                            { value: 0, label: "不显示" }
                        ]}
                        onChange={(v) => cs.onChange({ boxShadow: v })}
                    />
                </SettingRow>
            ) : null}
        </>
    );
}

/** 扩展设置页签内容（原版 ModuleStyleEdit + ContentStyleEdit 组合） */
export function ExtensionStyleSettings({
    data,
    onChange,
    type = "",
    topPaddingMax,
    showPaddingTop,
    paddingTopMax
}: NestedProps & { type?: string; topPaddingMax?: number; showPaddingTop?: boolean; paddingTopMax?: number }) {
    return (
        <>
            <ModuleStyleSettingsField data={data} onChange={onChange} type={type} topPaddingMax={topPaddingMax} />
            <ContentStyleSettingsField
                data={data}
                onChange={onChange}
                type={type}
                showPaddingTop={showPaddingTop}
                paddingTopMax={paddingTopMax}
            />
        </>
    );
}

// ---------------- 分组折叠卡列表（各模块 src/*GroupEdit.vue 的通用形态） ----------------

/**
 * 折叠卡片列表壳（对齐 CategoryGroupEdit/GroupListEdit 的 .demo-collapse 形态）：
 * 卡 margin-top 12px、radius 2px、阴影 0 0 4px rgba(10,42,97,.2)，头 bg #f7f8fa、
 * 删除徽标 -10px 18px 圆形 #bbb，底部整宽 .dec-pic-add-btn 添加按钮。
 * 原版为拖拽排序，本实现以卡头「上移/下移」文字钮承接（同 PicList 口径）。
 */
export function GroupCardList({
    items,
    renderItem,
    onAdd,
    onRemove,
    onMove,
    addText,
    collapsible = true,
    canRemove = true
}: {
    items: any[];
    renderItem: (item: any, index: number) => React.ReactNode;
    onAdd: () => void;
    onRemove: (index: number) => void;
    onMove: (index: number, delta: number) => void;
    addText: string;
    collapsible?: boolean;
    canRemove?: boolean;
}) {
    const { t } = useI18n();
    const [openIndex, setOpenIndex] = useState(0);
    return (
        <div style={{ padding: "0 10px 15px" }}>
            {items.map((item, index) => {
                const open = !collapsible || openIndex === index;
                return (
                    <div
                        key={index}
                        style={{
                            width: "100%",
                            position: "relative",
                            marginTop: 12,
                            borderRadius: 2,
                            backgroundColor: "#fff",
                            boxShadow: "0 0 4px 0 rgba(10, 42, 97, 0.2)"
                        }}
                    >
                        <div
                            style={{
                                backgroundColor: "#f7f8fa",
                                padding: "10px",
                                fontSize: 14,
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "space-between",
                                cursor: collapsible ? "pointer" : "default"
                            }}
                            onClick={() => collapsible && setOpenIndex(open ? -1 : index)}
                        >
                            <span>{`第【${index + 1}】组`}</span>
                            <span style={{ display: "flex", gap: 10, fontSize: 12 }} onClick={(e) => e.stopPropagation()}>
                                <a className={index === 0 ? "is-disabled" : ""} onClick={() => onMove(index, -1)}>
                                    {t('上移')}
                                </a>
                                <a className={index === items.length - 1 ? "is-disabled" : ""} onClick={() => onMove(index, 1)}>
                                    {t('下移')}
                                </a>
                            </span>
                        </div>
                        {open ? <div>{renderItem(item, index)}</div> : null}
                        {canRemove ? (
                            <span
                                title={t('删除')}
                                onClick={() => onRemove(index)}
                                style={{
                                    position: "absolute",
                                    right: -10,
                                    top: -10,
                                    color: "#fff",
                                    background: "#bbb",
                                    border: "2px solid #fff",
                                    borderRadius: "50%",
                                    zIndex: 2,
                                    width: 18,
                                    height: 18,
                                    textAlign: "center",
                                    lineHeight: "16px",
                                    cursor: "pointer",
                                    fontSize: 12,
                                    boxSizing: "border-box"
                                }}
                            >
                                ×
                            </span>
                        ) : null}
                    </div>
                );
            })}
            <a className="dec-pic-add-btn" onClick={onAdd} style={{ marginTop: 12 }}>
                {t(addText)}
            </a>
        </div>
    );
}

// ---------------- 缓动效果（animation/Edit.vue） ----------------

export function AnimationSettingsField({ data, onChange, type = "categoryA1" }: NestedProps & { type?: string }) {
    const an = nested(data, onChange, "animation");
    const sub = an.data;
    const animType = String(sub.type ?? "scale");
    const options = [
        { value: "scale", label: "放大" },
        ...(type !== "categoryA1" ? [{ value: "flip", label: "翻转" }] : []),
        { value: "slideLeft", label: "左入" },
        { value: "slideRight", label: "右入" },
        { value: "slideTop", label: "上滑" },
        { value: "close", label: "关闭" }
    ];
    return (
        <>
            <div className="dec-edit-group dec-edit-group-block">
                <div className="dec-edit-group-title">
                    <div className="label">图片缓动效果</div>
                </div>
                <div className="dec-edit-group-con">
                    <SegmentedRadio value={animType} options={options} onChange={(v) => an.onChange({ type: v })} />
                </div>
            </div>
            {animType !== "close" ? (
                <>
                    <RangeField {...an} field="duration" label="缓动效果速度" max={3} step={0.1} />
                    <RangeField {...an} field="delay" label="缓动效果时长" max={2} step={0.1} />
                </>
            ) : null}
            {animType !== "flip" && animType !== "close" ? (
                <RangeField {...an} field="range" label="缓动效果幅度" max={2} step={0.1} />
            ) : null}
        </>
    );
}

// ---------------- 小标题（commonSubTitle/Edit.vue） ----------------

export function SubTitleSettingsField({
    data,
    onChange,
    field = "subTitle",
    title,
    type = ""
}: NestedProps & { field?: string; title: string; type?: string }) {
    const st = nested(data, onChange, field);
    const sub = st.data;
    const show = Number(sub.showTitle ?? 1) === 1;
    return (
        <>
            {type !== "categoryRec" ? (
                <SettingRow label={`${title}文字`}>
                    <SegmentedRadio
                        value={Number(sub.showTitle ?? 1)}
                        options={[
                            { value: 1, label: "显示" },
                            { value: 0, label: "不显示" }
                        ]}
                        onChange={(v) => st.onChange({ showTitle: v })}
                    />
                </SettingRow>
            ) : null}
            {show ? (
                <>
                    <ColorField {...st} field="titleColor" label={`${title}颜色`} resetValue={defaultSubTitle.titleColor} />
                    <SettingRow label={`${title}大小`}>
                        <SegmentedRadio
                            value={Number(sub.titleFontSize ?? 12)}
                            options={[
                                { value: 12, label: "小(12号)" },
                                { value: 14, label: "中(14号)" },
                                { value: 16, label: "大(16号)" }
                            ]}
                            onChange={(v) => st.onChange({ titleFontSize: v })}
                        />
                    </SettingRow>
                    <SettingRow label={`${title}粗细`}>
                        <SegmentedRadio
                            value={Number(sub.titleFontBold ?? 400)}
                            options={[
                                { value: 400, label: "常规体" },
                                { value: 700, label: "加粗体" }
                            ]}
                            onChange={(v) => st.onChange({ titleFontBold: v })}
                        />
                    </SettingRow>
                </>
            ) : null}
        </>
    );
}
