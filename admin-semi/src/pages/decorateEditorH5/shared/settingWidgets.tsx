/**
* 模块设置面板通用控件（Semi/React 版，壳层对齐 view/admin 的 dec-* 体系）
*
* 结构/数值口径（原版 src/css/decorate.less + 各共享 Edit 组件）：
* - 字段行 `.dec-edit-group`（padding 10px 16px；标签.label 14px #969799 margin-right 16px；控件区.dec-edit-group-con flex:1 右对齐）
* - 分组标题三形态：dec-spread-title（平面粗体）/ dec-edit-title-box（灰带 + 3px 蓝条）/ dec-edit-section-title
* - 单选组：以 PC 编辑器为准（用户裁定 2026-11-19）——模块字段用普通 Semi RadioGroup
* 圆点形态（同 PcDecorateEditor FieldControl 的 kind:'radio' 写法，无 type）；仅页面设置
* 用 Semi RadioGroup type="button" 分段形态（同 PC PageSettings），见 SegmentedButtonRadio
* - 滑杆 `.dec-range-group`：共享 SliderWithInput（Semi Slider + InputNumber，同 PC 编辑器）
* - 颜色行：共享 components/ColorField 外壳（「重置」链 + AlphaColorInput，预置色为原版 SelectColor 14 色）
* - 图片：单图走共享 ImageUploader、列表走 GalleryPickerModal（素材库口径，同 PC 编辑器；原 Semi Upload 直传已弃）
* - 链接：共享 LinkField（components/LinkPicker），经 PicLinkField 适配——回写保留原链接
* 对象全部字段（原版 SelectLink 的完整 LinkType：path/label/name/data 等），只更新 link/title
* - 图片列表 `.dec-pic-group`：60×60 contain 图盒 + 底部 20px「更换图片」条 + hover 删除徽标 + 信息行 + 整宽添加按钮 + 上移/下移
*
* 统一 onChange(patch) 协议：所有控件只上报字段级 patch，由 store 浅合并进模块数据。
*
* PicItemEditor（2026-11-20 新增）：PicListField 的单条目编辑（图盒/标题/描述/链接/
* 排序行/删除徽标）抽为共享组件，PicListField 内部改用它（既有使用方 DOM 与行为
* 不变，图库弹窗仍由 PicListField 统一托管）；单图槽位模块（图片魔方 imageSquareAd）
* 可直接复用它编辑 picList[picIndex]。
*
* ProductPicItemEditor（2026-11-21 晨新增、同日晚间已移除）：曾为单图槽位编辑器
* （卡面外挂「从商品选择图片」工具条形态），用户 2026-11-21 定版纠正落点后废弃——
* 图片魔方改调共享图片列表编辑器（PicListEditor，components/PicListEditor，
* 自 PcDecorateEditor 抽取升格，与 PC 编辑器同一共享件）；本文件仅保留
* productPicPatch（魔方在调用点做商品批量落位与 H5 LinkType 链接适配时用）。
*/
import { useState} from "react";
import { Input, Radio, RadioGroup, Select, Switch, Toast} from "@douyinfe/semi-ui";
import { IconPlus} from "@douyinfe/semi-icons";
import AlphaColorInput from "../../../components/AlphaColorInput";
import SliderWithInput from "../../../components/SliderWithInput";
import SharedColorField from "../../../components/ColorField";
import ImageUploader from "../../../components/ImageUploader";
import GalleryPickerModal from "../../../components/GalleryPickerModal";
import PicListEditor from "../../../components/PicListEditor";
import BuyBtnStylePicker, { H5_BUY_BTN_STYLE_OPTIONS, H5_BUY_BTN_TYPE_OPTIONS, type BuyBtnStyleOption } from "../../../components/BuyBtnStylePicker";
import { type PickedImage} from "../../../components/ProductImagePicker";
import FloorIcon from "../../../components/FloorIcon";
import { LinkField, type LinkValue} from "../../../components/LinkPicker";
import { imageFormat, pickPic} from "./format";
import AddItemButton from "./AddItemButton";
import { defaultTitle, defaultFrame} from "../contract/sharedDefaults";

import { useI18n} from '../../../i18n';
type Patch = Record<string, any>;
interface FieldProps {
data: Record<string, any>;
onChange: (patch: Patch) => void;
}

// ---------------- 布局 ----------------

/** 分组标题形态：box=dec-edit-title-box（默认）/ spread=dec-spread-title / section=dec-edit-section-title */
export type SettingGroupVariant = "box" | "spread" | "section";

export function SettingGroup({ title, children, variant = "box"}: { title?: string; children: React.ReactNode; variant?: SettingGroupVariant}) {
const { t} = useI18n();
return (
<div className="h5-editor-setting-group">
{title? (
variant === "spread"? (
<div className="dec-spread-title">
<div className="title">{t(title)}</div>
</div>
): variant === "section"? (
<div className="dec-edit-section-title">{t(title)}</div>
): (
<div className="dec-edit-title-box">
<div className="title">{t(title)}</div>
</div>
)
): null}
{children}
</div>
);
}

/** 平面分组标题（原版.dec-spread-title） */
export function DecSpreadTitle({ title}: { title: string}) {
const { t} = useI18n();
return (
<div className="dec-spread-title">
<div className="title">{t(title)}</div>
</div>
);
}

/** 分隔线（原版.dec-divider-line） */
export function DecDivider() {
return <div className="dec-divider-line" />;
}

/** 字段行（原版.dec-edit-group）：左标签 + 右控件区 */
export function SettingRow({ label, children, block = false}: { label: string; children: React.ReactNode; block?: boolean}) {
const { t} = useI18n();
return (
<div className={`dec-edit-group${block? " dec-edit-group-block": ""}`}>
<div className="dec-edit-group-title">
<div className="label">{label? t(label): ""}</div>
</div>
<div className="dec-edit-group-con">{children}</div>
</div>
);
}

// ---------------- 单选组（以 PC 编辑器为准，用户裁定 2026-11-19） ----------------

interface RadioOption {
value: number | string;
label: string;
}

/**
* 选中态/回写值归一：按 String 比较命中选项（容忍存量数据里 "1"/1 混写），
* 命中时一律使用选项的原始类型值——字符串选项（"normal"/"bold"/"500" 等）
* 绝不做 Number() 转换，数字选项也保持数字。
*/
function matchRadioOption(value: any, options: RadioOption[]): RadioOption | undefined {
return options.find((o) => String(o.value) === String(value));
}

function radioChangeValue(raw: any, options: RadioOption[]): number | string {
const matched = matchRadioOption(raw, options);
return matched? matched.value: raw;
}

/** 裸单选组（无字段行外壳，供 dec-edit-group-block 等容器内嵌使用）
* 模块字段口径（用户裁定 2026-11-19 以 PC 编辑器为准）：普通 Semi RadioGroup
* 圆点形态，无 type——同 PcDecorateEditor FieldControl 的 kind:'radio' 写法。
* 函数名沿用 SegmentedRadio（近百处模块调用点不改名），仅形态变更。 */
export function SegmentedRadio({ value, options, onChange}: { value: any; options: RadioOption[]; onChange: (v: number | string) => void}) {
const { t} = useI18n();
const matched = matchRadioOption(value, options);
return (
<RadioGroup
value={matched? matched.value: value}
onChange={(e: any) => onChange(radioChangeValue(e?.target?.value, options))}
>
{options.map((o) => (
<Radio key={String(o.value)} value={o.value}>
{t(o.label)}
</Radio>
))}
</RadioGroup>
);
}

/** 单选（无标签行，整行单选组） */
export function RadioField({ data, onChange, field, options}: FieldProps & { field: string; options: RadioOption[]}) {
return (
<div className="dec-edit-group">
<div className="dec-edit-group-con">
<SegmentedRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v})} />
</div>
</div>
);
}

/** 带标题的单选 */
export function RadioRowField({ data, onChange, field, label, options}: FieldProps & { field: string; label: string; options: RadioOption[]}) {
return (
<SettingRow label={label}>
<SegmentedRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v})} />
</SettingRow>
);
}

// ---------------- 购买按钮样式（可视化点选，2026-11-21 用户口径） ----------------

/**
 * 购买按钮样式字段族：与 RadioField/RadioRowField 同接口（data/onChange/field），
 * 控件本体为共享 BuyBtnStylePicker（直接显示按钮图标点选，形态对齐全局
 * 「购买按钮样式设置」页）。只换形态：回写仍是 { [field]: 选项原始数值 }，
 * 字段值/默认值/数据形状逐字不变。
 * - 缺省 options = 数字型 buyBtnStyle 0–8（H5_BUY_BTN_STYLE_OPTIONS，unified 视觉集）；
 * - 对象型 buyBtnStyle 的 btnType 子字段传 visualSet="h5BtnType"（选项自动换
 *   H5_BUY_BTN_TYPE_OPTIONS 1–5），并把该对象的 iconImage 传 customIcon 供
 *   「自定义图片」项预览。
 */
interface BuyBtnStyleFieldProps {
    visualSet?: "unified" | "h5BtnType";
    options?: BuyBtnStyleOption[];
    customIcon?: unknown;
}

function buyBtnOptionsFor(props: BuyBtnStyleFieldProps): BuyBtnStyleOption[] {
    if (props.options) return props.options;
    return props.visualSet === "h5BtnType" ? H5_BUY_BTN_TYPE_OPTIONS : H5_BUY_BTN_STYLE_OPTIONS;
}

/** 购买按钮样式（裸控件，供 SettingRow block 等容器内嵌，与 SegmentedRadio 同位） */
export function BuyBtnStyleRadio({ value, onChange, visualSet, options, customIcon }: { value: any; onChange: (v: number) => void } & BuyBtnStyleFieldProps) {
    return (
        <BuyBtnStylePicker
            value={value}
            options={buyBtnOptionsFor({ visualSet, options })}
            visualSet={visualSet}
            customIcon={customIcon}
            onChange={onChange}
        />
    );
}

/** 购买按钮样式（无标签行，整行控件，与 RadioField 同位） */
export function BuyBtnStyleField({ data, onChange, field, visualSet, options, customIcon }: FieldProps & { field: string } & BuyBtnStyleFieldProps) {
    return (
        <div className="dec-edit-group">
            <div className="dec-edit-group-con">
                <BuyBtnStyleRadio value={data?.[field]} visualSet={visualSet} options={options} customIcon={customIcon} onChange={(v) => onChange({ [field]: v })} />
            </div>
        </div>
    );
}

/** 购买按钮样式（带标题，整宽块形态——可视化网格占整行，与原版整宽块口径一致） */
export function BuyBtnStyleRowField({ data, onChange, field, label, visualSet, options, customIcon }: FieldProps & { field: string; label: string } & BuyBtnStyleFieldProps) {
    return (
        <SettingRow label={label} block>
            <BuyBtnStyleRadio value={data?.[field]} visualSet={visualSet} options={options} customIcon={customIcon} onChange={(v) => onChange({ [field]: v })} />
        </SettingRow>
    );
}

/** 裸分段单选组（Semi RadioGroup type="button"）：仅页面设置使用——
 *  同 PC 编辑器 PageSettings 的头部风格/背景图显示/背景图填充大小写法。
 *  模块字段禁用此控件（走上面的圆点 SegmentedRadio）。
 *  宽度口径（2026-11-19 用户反馈「太宽了」后对齐 PC）：PC PageSettings 的
 *  同名控件按内容自然宽度排列、无任何拉伸；本控件早先带 flex:1 行内样式、
 *  又被 editor.less 的分段均分规则撑满控件区（实测轨道 267px、每项 133px，
 *  而 PC 口径单项仅约 76px、整组约 160px）。现去掉行内拉伸并挂专属类
 *  dec-seg-btn-auto，由 editor.less 对该类复位 width:auto / 子项 flex:none；
 *  图标样式单选 IconStyleRadioField 不挂此��、保留全宽均分形态。 */
export function SegmentedButtonRadio({ value, options, onChange }: { value: any; options: RadioOption[]; onChange: (v: number | string) => void }) {
    const { t } = useI18n();
    const matched = matchRadioOption(value, options);
    return (
        <RadioGroup
            type="button"
            className="dec-seg-btn-auto"
            // 选中态兜底（2026-11-19 实测发现）：存量页面数据里背景图显示/填充
            // 大小存的是空串 ""（如页面 10020），调用方 Number("")=0 命中不了
            // 任何选项，整组无选中、蓝色胶囊不出现（标签行却按兜底文案显示
            // 第一项，两者对不上）。现未命中时按第一项显示——同 PC
            // PageSettings 对头部风格的口径（非 2 即 1）；仅显示兜底，
            // 不回写数据，用户点选后才按原契约写入选项值。
            value={matched ? matched.value : options[0]?.value}
            onChange={(e: any) => onChange(radioChangeValue(e?.target?.value, options))}
        >
            {options.map((o) => (
                <Radio key={String(o.value)} value={o.value}>
                    {t(o.label)}
                </Radio>
            ))}
        </RadioGroup>
    );
}

/** 带标题的分段单选（仅页面设置使用，见 SegmentedButtonRadio） */
export function SegmentedButtonRadioRowField({ data, onChange, field, label, options }: FieldProps & { field: string; label: string; options: RadioOption[] }) {
    return (
        <SettingRow label={label}>
            <SegmentedButtonRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v })} />
        </SettingRow>
    );
}

/**
 * 图标样式单选（原版 BannerSearch 的 el-radio-button + iconfont 字形选项形态）：
 * 带 iconpark 的选项以 IconPark 字形呈现（FloorIcon 内嵌 SVG，描边随 currentColor——
 * 未选时为深色描边、选中胶囊填充时随字色变白），其余选项（自定义/不显示）走文字。
 * 选中态归一同 SegmentedRadio（String 比较，容忍存量 "1"/1 混写）。
 * 本控件是图标选择器，形态例外保留 type="button" 分段（同 PC 编辑器图标单选的按钮组形态），
 * 不随模块字段的圆点口径变更。
 * 字形映射由调用方模块的共享映射表提供（如 banner 的 iconStyles.ts），与预览/前台同源。
 */
export function IconStyleRadioField({
    data,
    onChange,
    field,
    label,
    options
}: FieldProps & { field: string; label: string; options: (RadioOption & { iconpark?: string })[] }) {
    const { t } = useI18n();
    const value = data?.[field];
    const matched = options.find((o) => String(o.value) === String(value));
    return (
        <SettingRow label={label}>
            <RadioGroup
                type="button"
                value={matched ? matched.value : value}
                style={{ flex: 1, minWidth: 0 }}
                onChange={(e: any) => onChange({ [field]: radioChangeValue(e?.target?.value, options) })}
            >
                {options.map((o) => (
                    <Radio key={String(o.value)} value={o.value}>
                        {o.iconpark ? (
                            <span title={t(o.label)} style={{ display: "inline-flex" }}>
                                <FloorIcon value={`iconpark:${o.iconpark}`} size={24} />
                            </span>
                        ) : (
                            t(o.label)
                        )}
                    </Radio>
                ))}
            </RadioGroup>
        </SettingRow>
    );
}

// ---------------- 滑杆（原版 .dec-range-group：el-slider show-input 无步进钮） ----------------

export function RangeField({
    data,
    onChange,
    field,
    label,
    min = 0,
    max = 30,
    step = 1
}: FieldProps & { field: string; label: string; min?: number; max?: number; step?: number }) {
    const value = Number(data?.[field] ?? 0);
    return (
        <SettingRow label={label}>
            <div className="dec-range-group">
                <SliderWithInput value={value} min={min} max={max} step={step} onChange={(v) => onChange({ [field]: v })} />
            </div>
        </SettingRow>
    );
}

/** 数值字段：统一走滑杆形态（原版 el-slider show-input）；max 由调用方按字段传，frame 类字段由 FrameSettingsField 传 30 */
export function NumberField({
    data,
    onChange,
    field,
    label,
    min = 0,
    max = 999,
    step = 1,
    suffix
}: FieldProps & { field: string; label: string; min?: number; max?: number; step?: number; suffix?: string }) {
    const value = Number(data?.[field] ?? 0);
    return (
        <SettingRow label={label}>
            <div className="dec-range-group">
                <SliderWithInput value={value} min={min} max={max} step={step} onChange={(v) => onChange({ [field]: v })} />
                {suffix ? <span className="dec-range-suffix">{suffix}</span> : null}
            </div>
        </SettingRow>
    );
}

// ---------------- 颜色（原版 .dec-color-group：重置链 + SelectColor） ----------------

/**
 * 颜色字段：左侧「重置」文字链（回到 resetValue，默认空=跟随主题）+ 方形色块触发器。
 * 弹层继续用 AlphaColorInput 的取色面板（预置色已换成原版 SelectColor 的 14 色）。
 */
export function ColorField({
    data,
    onChange,
    field,
    label,
    resetValue = "",
    allowEmpty = true
}: FieldProps & { field: string; label: string; resetValue?: string; allowEmpty?: boolean }) {
    return (
        <SettingRow label={label}>
            <SharedColorField
                value={String(data?.[field] ?? "")}
                resetValue={resetValue}
                allowEmpty={allowEmpty}
                onChange={(v) => onChange({ [field]: v })}
            />
        </SettingRow>
    );
}

/** 色板单选组（原版 .dec-radio-color-group：24px 圆形色板，选中边框 #155bd4） */
export function ColorSwatchRadioField({
    data,
    onChange,
    field,
    label,
    colors
}: FieldProps & { field: string; label: string; colors: string[] }) {
    const value = String(data?.[field] ?? "");
    return (
        <SettingRow label={label}>
            <div className="dec-radio-color-group" style={{ display: "flex", alignItems: "center" }}>
                {colors.map((color) => (
                    <span
                        key={color}
                        className={`dec-radio-button${value.toLowerCase() === color.toLowerCase() ? " selected" : ""}`}
                        onClick={() => onChange({ [field]: color })}
                    >
                        <span className="dec-radio-color-button-con">
                            <i style={{ backgroundColor: color }} />
                        </span>
                    </span>
                ))}
            </div>
        </SettingRow>
    );
}

// ---------------- 文本 / 开关 / 下拉 ----------------

export function TextField({ data, onChange, field, label, placeholder }: FieldProps & { field: string; label: string; placeholder?: string }) {
    const { t } = useI18n();
    return (
        <SettingRow label={label}>
            <Input
                value={data?.[field] ?? ""}
                placeholder={placeholder ? t(placeholder) : undefined}
                style={{ width: 240, maxWidth: "100%" }}
                onChange={(v: string) => onChange({ [field]: v })}
            />
        </SettingRow>
    );
}

export function SwitchField({ data, onChange, field, label }: FieldProps & { field: string; label: string }) {
    return (
        <SettingRow label={label}>
            <Switch checked={Number(data?.[field]) === 1} onChange={(v: boolean) => onChange({ [field]: v ? 1 : 0 })} />
        </SettingRow>
    );
}

export function SelectField({ data, onChange, field, label, options }: FieldProps & { field: string; label: string; options: RadioOption[] }) {
    const { t } = useI18n();
    return (
        <SettingRow label={label}>
            <Select
                value={data?.[field]}
                style={{ width: 240, maxWidth: "100%" }}
                optionList={options.map((o) => ({ ...o, label: t(o.label) })) as any}
                onChange={(v: any) => onChange({ [field]: v })}
            />
        </SettingRow>
    );
}

// ---------------- 图片（原版 PicSelect / PicList） ----------------

/**
 * 单图选择（2026-11-15 换口径）：改用 PC 同款 ImageUploader（single）——其内部即
 * GalleryPickerModal，点击打开素材库弹窗（弹窗内可上传新图），与 PC 编辑器一致。
 * 此前实现是 Semi Upload 直传：点击路径只依赖隐藏的原生 file input 唤起系统文件框，
 * 页面内零反馈、选过的图不能复用，且线上实测出现点击无任何反应；
 * 换成素材库弹窗后点击必有页面内响应。value/onChange 契约不变（{ picUrl, picThumb }）。
 */
export function PicSelectBox({
    pic,
    onSelect,
    onReset
}: {
    pic?: { picUrl?: string; picThumb?: string } | null;
    onSelect: (pic: { picUrl: string; picThumb: string }) => void;
    onReset?: () => void;
}) {
    const { t } = useI18n();
    const url = pic ? pickPic(pic) : "";
    return (
        <div className="dec-pic-select">
            <ImageUploader
                single
                value={url ? [imageFormat(url)] : []}
                onChange={(urls) => {
                    const u = urls[0] || "";
                    onSelect({ picUrl: u, picThumb: u });
                }}
            />
            {onReset ? (
                <a className="dec-pic-reset" onClick={onReset}>
                    {t('重置')}
                </a>
            ) : null}
        </div>
    );
}

/**
 * 链接字段适配（对齐原版 SelectLink 的 LinkType 口径）：
 * 存量链接对象是完整 LinkType（path/label/name/link/appLink/data/id/sn 等，前台
 * urlFormat 按 path+data 消费），回写时必须保留原对象全部字段，只更新本控件
 * 负责的 link/title 两键——旧实现把对象压成 { link, title }，重选一次即丢
 * path/data 等键，系统链接（商品/分类等）随之失效。
 * 对外 props 不变（value/onChange 形状同前），仅内部行为修复：
 * - 喂入 LinkField 时透传原对象全部键（存量嵌套 { link: { link, title } } 占位
 *   形状先拍平一层供显示/回显），name 缺失时以 title 兜底显示；
 * - 选择器返回后以「原对象 ← 选择结果」合并回写，link/title 按选择结果更新；
 * - 清空时回写 { link: "", title: "" } 空契约（同 PicList 新项默认形状）。
 */
export function PicLinkField({
    value,
    onChange
}: {
    value?: { link?: string; title?: string } | null;
    onChange: (v: { link: string; title: string }) => void;
}) {
    const raw: Record<string, any> = value && typeof value === "object" ? (value as Record<string, any>) : {};
    // 存量占位形状 { link: { link, title }, linkTitle }：内层对象拍平一层供选择器回显
    const inner: Record<string, any> | null = raw.link && typeof raw.link === "object" ? raw.link : null;
    const url = typeof raw.link === "string" ? raw.link : inner?.link || raw.path || raw.appLink || "";
    const hasValue = !!(url || raw.path || raw.label || raw.name || raw.title || inner?.title);
    const linkValue: LinkValue | undefined = hasValue
        ? { ...raw, ...(inner ?? {}), link: url, name: raw.name || raw.title || inner?.title || "" }
        : undefined;
    return (
        <LinkField
            value={linkValue}
            onChange={(picked) => {
                if (!picked || Object.keys(picked).length === 0) {
                    onChange({ link: "", title: "" });
                    return;
                }
                const out: Record<string, any> = { ...raw, ...picked };
                out.link = picked.link || picked.path || (typeof raw.link === "string" ? raw.link : "") || "";
                out.title = picked.name || picked.label || raw.title || "";
                onChange(out as { link: string; title: string });
            }}
        />
    );
}

/**
 * 单图字段行（标签 + PicSelect 图盒）。
 * resetPic：原版 PicSelect 的 :default-value——点「重置」回该默认图对象（原版
 * removePic 写回 { ...defaultValue }）；不传时维持旧行为（重置为清空）。
 */
export function PicSelectField({
    data,
    onChange,
    field,
    label,
    resetPic
}: FieldProps & { field: string; label: string; resetPic?: { picUrl?: string; picThumb?: string } }) {
    const pic = data?.[field];
    const picObj = pic && typeof pic === "object" ? pic : typeof pic === "string" && pic ? { picUrl: pic, picThumb: pic } : null;
    return (
        <SettingRow label={label}>
            <PicSelectBox
                pic={picObj}
                onSelect={(p) => onChange({ [field]: p })}
                onReset={() => onChange({ [field]: resetPic ? { ...resetPic } : { picUrl: "", picThumb: "" } })}
            />
        </SettingRow>
    );
}

/** 单图编辑器（块级：标签行 + 图盒，原 PicSelect 形态）；resetPic 语义同 PicSelectField */
export function SinglePicField({
    data,
    onChange,
    field,
    label,
    resetPic
}: FieldProps & { field: string; label: string; resetPic?: { picUrl?: string; picThumb?: string } }) {
    const pic = data?.[field];
    const picObj = pic && typeof pic === "object" ? pic : typeof pic === "string" && pic ? { picUrl: pic, picThumb: pic } : null;
    return (
        <div className="dec-edit-group dec-edit-group-block">
            <div className="dec-edit-group-title">
                <div className="label">{label}</div>
            </div>
            <div className="dec-edit-group-con">
                <div className="dec-pic-group dec-picSingle-group">
                    <div className="dec-pic-group-item">
                        <div className="dec-pic-group-item-con">
                            <PicSelectBox
                                pic={picObj}
                                onSelect={(p) => onChange({ [field]: p })}
                                onReset={() => onChange({ [field]: resetPic ? { ...resetPic } : { picUrl: "", picThumb: "" } })}
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}

// ---------------- 图片列表（原版 PicList） ----------------

export interface PicItem {
    picUrl?: string;
    picThumb?: string;
    picTitle?: string;
    picDesc?: string;
    picLink?: { link?: string; title?: string };
    [key: string]: any;
}

/**
 * 单图条目编辑器（2026-11-20 自 PicListField 条目抽出）：图盒（60×60 contain +
 * 底部「更换图片」条）+ 信息行（标题/描述/链接）+ 可选排序行（上移/下移）+ 可选
 * 删除徽标，DOM/类名与 PicListField 条目逐一致。
 * - onPickImage 存在时点击图盒走外部托管的图库弹窗（PicListField 的追加/替换
 *   共用弹窗即此口径）；缺省时组件自带 GalleryPickerModal 单选弹窗，回写
 *   { picUrl, picThumb }（保留条目其余字段）——单图槽位模块（图片魔方）用此形态。
 * - onRemove 存在才渲染 del-item；单图槽位场景由调用方传「清空槽位」语义的回调
 *   （原版 PicList 单图模式 removePhoto 清空图对象、槽位保留）。
 * - showAddBox=true 时空态图盒内渲染加号框（原版 PicList pic-add-box 口径）。
 */
export function PicItemEditor({
    item,
    onPatch,
    showText = true,
    showDesc = false,
    showLink = true,
    showImage = true,
    showAddBox = false,
    onPickImage,
    onRemove,
    onMove,
    isFirst = false,
    isLast = false
}: {
    item: PicItem;
    onPatch: (patch: Partial<PicItem>) => void;
    showText?: boolean;
    showDesc?: boolean;
    showLink?: boolean;
    showImage?: boolean;
    showAddBox?: boolean;
    onPickImage?: () => void;
    onRemove?: () => void;
    onMove?: (delta: number) => void;
    isFirst?: boolean;
    isLast?: boolean;
}) {
    const { t } = useI18n();
    const [pickerOpen, setPickerOpen] = useState(false);
    const handlePick = () => {
        if (onPickImage) onPickImage();
        else setPickerOpen(true);
    };
    return (
        <>
            {showImage ? (
                <div className="item-image">
                    <div className="item-image-con" onClick={handlePick}>
                        {pickPic(item) ? (
                            <img src={imageFormat(pickPic(item))} alt="" />
                        ) : showAddBox ? (
                            <span
                                className="pic-add-box"
                                style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center", color: "#999" }}
                            >
                                <IconPlus style={{ fontSize: 20 }} />
                            </span>
                        ) : null}
                        <span className="change-image">{t('更换图片')}</span>
                    </div>
                </div>
            ) : null}
            <div className="item-info">
                {showText ? (
                    <div className="item-info-item">
                        <span className="lable">{t('标题')}</span>
                        <Input
                            size="small"
                            placeholder={t('建议10个字以内，可不填')}
                            value={item.picTitle ?? ""}
                            onChange={(v: string) => onPatch({ picTitle: v })}
                        />
                    </div>
                ) : null}
                {showDesc ? (
                    <div className="item-info-item">
                        <span className="lable">{t('描述')}</span>
                        <Input
                            size="small"
                            placeholder={t('可不填')}
                            value={item.picDesc ?? ""}
                            onChange={(v: string) => onPatch({ picDesc: v })}
                        />
                    </div>
                ) : null}
                {showLink ? (
                    <div className="item-info-item">
                        <span className="lable">{t('链接')}</span>
                        <PicLinkField
                            value={item.picLink}
                            onChange={(v) => onPatch({ picLink: { ...(item.picLink || {}), ...v } })}
                        />
                    </div>
                ) : null}
                {onMove ? (
                    <div className="item-info-item dec-pic-sort-row">
                        <a className={isFirst ? "is-disabled" : ""} onClick={() => onMove(-1)}>
                            {t('上移')}
                        </a>
                        <a className={isLast ? "is-disabled" : ""} onClick={() => onMove(1)}>
                            {t('下移')}
                        </a>
                    </div>
                ) : null}
            </div>
            {onRemove ? (
                <span className="del-item" title={t('删除')} onClick={onRemove}>
                    ×
                </span>
            ) : null}
            {!onPickImage ? (
                <GalleryPickerModal
                    visible={pickerOpen}
                    multiple={false}
                    limit={1}
                    onCancel={() => setPickerOpen(false)}
                    onConfirm={(urls: string[]) => {
                        const u = urls[0];
                        if (u) onPatch({ picUrl: u, picThumb: u });
                        setPickerOpen(false);
                    }}
                />
            ) : null}
        </>
    );
}

/**
 * 商品直选落图卡的字段构造（PC 编辑器 PicListEditor 的 makeItem 口径在 H5 的等价形）：
 * 图 + 商品名作 picName/picTitle + 商品详情链接。
 * 链接不用 PC 的 { link: "/product/{id}" } 字符串——H5 前台 urlFormat 按 LinkType 的
 * path+data 消费，故按共享 LinkPicker 选商品的形状构造（path/label/name/id/sn/data），
 * 并与 PicLinkField 的回写合并逐键一致（在原 picLink 上合并、link 取 path、title 取 name），
 * 保证经此组件选商品与经链接选择器选商品存下的数据完全同形。
 */
export function productPicPatch(current: PicItem, picked: PickedImage, productLabel: string): Partial<PicItem> {
    const linkValue: Record<string, any> = {
        path: "product",
        label: productLabel,
        name: picked.productName,
        id: picked.productId,
        sn: picked.productSn,
        data: { id: picked.productId, sn: picked.productSn }
    };
    return {
        picUrl: picked.url,
        picThumb: picked.url,
        picName: picked.productName,
        picTitle: picked.productName,
        picLink: { ...(current?.picLink || {}), ...linkValue, link: linkValue.path, title: linkValue.name }
    };
}

/**
 * 图片列表编辑器（原版 PicList 形态）：
 * 每项 = 60×60 contain 图盒（底部 20px「更换图片」条）+ 信息行（标题/描述/链接）+ hover 删除徽标 + 上移/下移；
 * 底部整宽共享 AddItemButton 添加按钮（2026-11-21 样式统一，取代旧 `.dec-pic-add-btn` 蓝框样式）。showText/showDesc/showLink/showAddButton 默认同原版（true/false/true/true）。
 * showImage 同原版 PicList（默认 true）：false 时不渲染图盒、添加按钮直接追加空文字项不开图库
 * （原版 onAdd 的 showImage=false 分支；BannerNav 导航文字即此口径——只有文字+链接、无图片字段）。
 * 条目编辑走共享 PicItemEditor（2026-11-20 抽出），图库弹窗仍由本组件统一托管
 * （追加多选/替换单选），既有使用方行为不变。
 */
export function PicListField({
    data,
    onChange,
    field,
    label,
    max = 20,
    showText = true,
    showDesc = false,
    showLink = true,
    showImage = true,
    showAddButton = true,
    addText = "添加图片"
}: FieldProps & {
    field: string;
    label: string;
    max?: number;
    showText?: boolean;
    showDesc?: boolean;
    showLink?: boolean;
    /** 原版 PicList 的 showImage：false 时每项无图盒且添加直接追加空文字项（导航文字等纯文字列表） */
    showImage?: boolean;
    /** 原版 PicList 的 showAddButton：false 时不渲染添加按钮（固定入口列表，如账号/订单入口） */
    showAddButton?: boolean;
    addText?: string;
}) {
    const { t } = useI18n();
    // 素材库弹窗状态：index=null 表示追加模式（可多选），否则为替换第 index 项（单选）
    const [pickerIndex, setPickerIndex] = useState<number | null>(null);
    const [pickerOpen, setPickerOpen] = useState(false);
    const list: PicItem[] = Array.isArray(data?.[field]) ? data[field] : [];

    const setList = (next: PicItem[]) => onChange({ [field]: next });
    const patchItem = (index: number, patch: Partial<PicItem>) => {
        const next = [...list];
        next[index] = { ...next[index], ...patch };
        setList(next);
    };
    const moveItem = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= list.length) return;
        const next = [...list];
        const [item] = next.splice(index, 1);
        next.splice(target, 0, item);
        setList(next);
    };

    return (
        <div className="dec-edit-group dec-edit-group-block">
            <div className="dec-edit-group-title">
                <div className="label">{t(label)}</div>
            </div>
            <div className="dec-edit-group-con">
                <div className="dec-pic-group">
                    {list.map((item, index) => (
                        <div key={index} className="dec-pic-group-item">
                            <div className="dec-pic-group-item-con">
                                <PicItemEditor
                                    item={item}
                                    onPatch={(patch) => patchItem(index, patch)}
                                    showText={showText}
                                    showDesc={showDesc}
                                    showLink={showLink}
                                    showImage={showImage}
                                    onPickImage={() => {
                                        setPickerIndex(index);
                                        setPickerOpen(true);
                                    }}
                                    onRemove={() => setList(list.filter((_, i) => i !== index))}
                                    onMove={(delta) => moveItem(index, delta)}
                                    isFirst={index === 0}
                                    isLast={index === list.length - 1}
                                />
                            </div>
                        </div>
                    ))}
                    {showAddButton && list.length < max && (
                        <AddItemButton
                            onClick={() => {
                                if (!showImage) {
                                    // 原版 PicList onAdd 的 showImage=false 分支：直接追加空文字项，不开图库
                                    setList([...list, { picTitle: "", picDesc: "", picLink: { link: "", title: "" } }]);
                                    return;
                                }
                                setPickerIndex(null);
                                setPickerOpen(true);
                            }}
                        >
                            {t(addText)}
                        </AddItemButton>
                    )}
                </div>
            </div>
            <GalleryPickerModal
                visible={pickerOpen}
                multiple={pickerIndex === null}
                limit={pickerIndex === null ? Math.max(max - list.length, 0) : 1}
                onCancel={() => setPickerOpen(false)}
                onConfirm={(urls) => {
                    if (pickerIndex !== null) {
                        const u = urls[0];
                        if (u) patchItem(pickerIndex, { picUrl: u, picThumb: u });
                    } else if (urls.length) {
                        setList([
                            ...list,
                            ...urls.map((u) => ({ picUrl: u, picThumb: u, picTitle: "", picDesc: "", picLink: { link: "", title: "" } }))
                        ]);
                    }
                    setPickerOpen(false);
                }}
            />
        </div>
    );
}

/**
 * 图片列表编辑（统一形态，2026-11-21 用户拍板整批迁移）：
 * 与 PicListField 同 props 的适配件——外层组标题外壳（dec-edit-group + label）
 * 与 PicListField 逐一致，内层改调共享图片列表编辑器（PicListEditor，
 * components/PicListEditor，与 PC 编辑器同一共享件）：卡片网格（拖拽排序，
 * 取代上移/下移）+ 点卡开「编辑图片」弹窗（选图框在左、标题/描述/链接纵列
 * 在右），素材中心选图与「从商品选择图片」两入口并列。
 * 数据契约与 PicListField 一致：只经 onChange({ [field]: next }) 回写整表；
 * 条目自有字段原样保留（PicListEditor 的 patch 为展开合并，不删字段）。
 * 商品批量落位（onPickManyProducts）：PicListEditor 不回传当前编辑位，落位
 * 口径定为「先按序填空图槽位、余者按 max 容量追加」；showText && showLink
 * 的列表走 productPicPatch 全量口径（图 + 商品名作标题 + 商品链接，H5
 * LinkType 形状），其余列表（无标题/无链接字段，如固定入口）只落图
 * （picUrl/picThumb），不写标题/链接。
 * showAddButton=false（固定入口列表）以 max=当前条数冻结添加（删除后条数
 * 同步收缩、添加口不出现），与 PicListField 不渲染添加按钮等价。
 * showImage=false（纯文字列表，如 banner 导航文字）走 PicListEditor 的无图
 * 条目模式（2026-11-21 补形态后经本件迁移）：卡片无缩略图、弹窗只有标题/
 * 描述/链接字段、空条目形状与 PicListField 的 showImage=false 添加形状逐字
 * 一致；无图模式下商品选图不启用（onPickManyProducts 不下传）。
 */
export function PicListEditorField({
    data,
    onChange,
    field,
    label,
    max = 20,
    showText = true,
    showDesc = false,
    showLink = true,
    showImage = true,
    showAddButton = true,
    addText = "添加图片"
}: FieldProps & {
    field: string;
    label: string;
    max?: number;
    showText?: boolean;
    showDesc?: boolean;
    showLink?: boolean;
    /** 原版 PicList 的 showImage：false 时走 PicListEditor 无图条目模式（纯文字+链接列表） */
    showImage?: boolean;
    /** false 时冻结添加（固定入口列表），与 PicListField 的 showAddButton 同义 */
    showAddButton?: boolean;
    addText?: string;
}) {
    const { t } = useI18n();
    const list: PicItem[] = Array.isArray(data?.[field]) ? data[field] : [];
    const setList = (next: PicItem[]) => onChange({ [field]: next });
    const effectiveMax = showAddButton ? max : list.length;
    const fullProductPatch = showText && showLink;

    const handlePickManyProducts = (items: PickedImage[]) => {
        if (!items.length) return;
        const next = [...list];
        let used = 0;
        // 先按序填空图槽位
        for (let i = 0; i < next.length && used < items.length; i += 1) {
            if (!String(next[i]?.picUrl || next[i]?.picThumb || "")) {
                next[i] = fullProductPatch
                    ? { ...next[i], ...productPicPatch(next[i], items[used], t('商品详情页')) }
                    : { ...next[i], picUrl: items[used].url, picThumb: items[used].url };
                used += 1;
            }
        }
        // 余者按容量追加（新条目形状：全量口径同 productPicPatch，只落图口径同 PicListField 图库追加形状）
        while (used < items.length && next.length < effectiveMax) {
            const picked = items[used];
            next.push(
                fullProductPatch
                    ? { ...productPicPatch({}, picked, t('商品详情页')) }
                    : { picUrl: picked.url, picThumb: picked.url, picTitle: "", picDesc: "", picLink: { link: "", title: "" } }
            );
            used += 1;
        }
        if (used) {
            setList(next);
            Toast.success(t('已带入 {count} 张商品图片', { count: used }));
        }
        if (used < items.length) {
            Toast.warning(t('最多只能添加 {max} 张图片', { max: effectiveMax }));
        }
    };

    return (
        <div className="dec-edit-group dec-edit-group-block">
            <div className="dec-edit-group-title">
                <div className="label">{t(label)}</div>
            </div>
            <div className="dec-edit-group-con">
                <PicListEditor
                    value={list as any}
                    onChange={(nextList) => setList(nextList as PicItem[])}
                    showText={showText}
                    showDesc={showDesc}
                    showLink={showLink}
                    showImage={showImage}
                    max={effectiveMax}
                    addText={addText}
                    onPickManyProducts={showImage ? handlePickManyProducts : undefined}
                />
            </div>
        </div>
    );
}

// ---------------- 模块标题（原版 CommonTitleEdit，19 项按 type 门控） ----------------

/**
 * 标题设置（对齐 src/modules/src/commonTitle/Edit.vue）：
 * 字段名与契约 title 对象一致（showTitle/titleStyle/titleAlign/titleBackground/titleBackground2/
 * titleBackgroundPic/titleRadius/titleText/titleColor/titleWeight/descText/descColor/showMore/
 * moreType/moreLink/moreColor/countdownLabel），老字段保持兼容。
 * type 门控：'title' 显示标题位置/标题粗细且隐藏显示标题栏/标题背景色/标题图片；
 * 'product' 多背景渐变色与风格三；'seckill2' 隐藏描述两项；其余为默认形态。
 * enableMoreType 时「显示查看更多」升级为「显示右侧内容」二选（更多链接/倒计时）。
 */
export function TitleSettingsField({
    data,
    onChange,
    field = "title",
    titleType = "",
    enableMoreType = false,
    label
}: FieldProps & { field?: string; titleType?: string; enableMoreType?: boolean; label?: string }) {
    const { t } = useI18n();
    const title = data?.[field] ?? {};
    const set = (patch: Patch) => onChange({ [field]: { ...title, ...patch } });
    const sub: FieldProps = { data: title, onChange: (patch: Patch) => set(patch) };
    const isTitle = titleType === "title";
    const isSeckill2 = titleType === "seckill2";
    const showMore = Number(title.showMore) === 1;
    const moreType = Number(title.moreType ?? 1);
    const moreLinkObj = title.moreLink && typeof title.moreLink === "object" && !Array.isArray(title.moreLink) ? title.moreLink : {};

    return (
        <>
            {!isTitle ? <DecDivider /> : null}
            <DecSpreadTitle title={label ?? (titleType === "categoryNavA3" ? "顶部标题设置" : "标题设置")} />
            {!isTitle ? (
                <RadioRowField
                    {...sub}
                    field="showTitle"
                    label="显示标题栏"
                    options={[
                        { value: 1, label: "显示" },
                        { value: 0, label: "不显示" }
                    ]}
                />
            ) : null}
            <RadioRowField
                {...sub}
                field="titleStyle"
                label="标题栏风格"
                options={
                    titleType === "product"
                        ? [
                              { value: 1, label: "风格一" },
                              { value: 2, label: "风格二" },
                              { value: 3, label: "风格三" }
                          ]
                        : [
                              { value: 1, label: "风格一" },
                              { value: 2, label: "风格二" }
                          ]
                }
            />
            {isTitle ? (
                <RadioRowField
                    {...sub}
                    field="titleAlign"
                    label="标题位置"
                    options={[
                        { value: 1, label: "居左" },
                        { value: 2, label: "居中" }
                    ]}
                />
            ) : null}
            {!isTitle ? <ColorField {...sub} field="titleBackground" label="标题背景色" resetValue={defaultTitle.titleBackground} /> : null}
            {titleType === "product" ? (
                <ColorField {...sub} field="titleBackground2" label="背景渐变色" resetValue={defaultTitle.titleBackground2} />
            ) : null}
            {!isTitle ? <PicSelectField {...sub} field="titleBackgroundPic" label="标题图片" /> : null}
            <RangeField {...sub} field="titleRadius" label="标题圆角" max={30} />
            <TextField {...sub} field="titleText" label="标题内容" placeholder="请输入标题内容" />
            <ColorField {...sub} field="titleColor" label="标题颜色" resetValue={defaultTitle.titleColor} />
            {isTitle ? (
                <RadioRowField
                    {...sub}
                    field="titleWeight"
                    label="标题粗细"
                    options={[
                        { value: "normal", label: "常规" },
                        { value: "500", label: "中粗" },
                        { value: "bold", label: "加粗" }
                    ]}
                />
            ) : null}
            {!isSeckill2 ? <TextField {...sub} field="descText" label="描述内容" placeholder="请输入标题内容" /> : null}
            {!isSeckill2 ? <ColorField {...sub} field="descColor" label="描述颜色" resetValue={defaultTitle.descColor} /> : null}
            <RadioRowField
                {...sub}
                field="showMore"
                label={enableMoreType ? "显示右侧内容" : "显示查看更多"}
                options={[
                    { value: 1, label: "显示" },
                    { value: 0, label: "不显示" }
                ]}
            />
            {enableMoreType ? (
                showMore ? (
                    <>
                        <RadioRowField
                            {...sub}
                            field="moreType"
                            label="右侧内容"
                            options={[
                                { value: 1, label: "显示更多" },
                                { value: 2, label: "显示倒计时" }
                            ]}
                        />
                        {moreType === 1 ? (
                            <>
                                <SettingRow label="链接文字">
                                    <Input
                                        value={moreLinkObj.title ?? ""}
                                        placeholder={t('更多')}
                                        style={{ width: 240, maxWidth: "100%" }}
                                        onChange={(v: string) => set({ moreLink: { ...moreLinkObj, title: v } })}
                                    />
                                </SettingRow>
                                <ColorField {...sub} field="moreColor" label="链接颜色" resetValue={defaultTitle.moreColor} />
                                <SettingRow label="链接地址">
                                    <PicLinkField value={moreLinkObj} onChange={(v) => set({ moreLink: { ...moreLinkObj, ...v } })} />
                                </SettingRow>
                            </>
                        ) : (
                            <>
                                <TextField {...sub} field="countdownLabel" label="前缀文字" placeholder="倒计时" />
                                <ColorField {...sub} field="moreColor" label="文字颜色" resetValue={defaultTitle.moreColor} />
                            </>
                        )}
                    </>
                ) : null
            ) : (
                <>
                    {!isSeckill2 ? <ColorField {...sub} field="moreColor" label="更多文字颜色" resetValue={defaultTitle.moreColor} /> : null}
                    <SettingRow label="更多链接">
                        <PicLinkField value={moreLinkObj} onChange={(v) => set({ moreLink: { ...moreLinkObj, ...v } })} />
                    </SettingRow>
                </>
            )}
        </>
    );
}

// ---------------- 模块边框（原版 CommonFrameEdit） ----------------

/**
 * 边框/结构设置（对齐 src/modules/src/commonFrame/Edit.vue）：
 * 默认形态 = 分隔线 + 「模块设置」平面标题 + 背景颜色/圆角/内边距/左右边距/上边距/下边距（滑杆 max 30）；
 * splitInnerPadding 时内边距拆为左右/上下两项；frameType='titleAd' 追加「左右内边距」(leftAndRightPadding)；
 * variant='notice'|'searchBar' 时改为专属 4 字段（文字颜色/组件背景/组件圆角 max50/组件高度 max50）
 * + 背景颜色/左右边距/上边距/下边距，且无分隔线与标题（同原版 type 门控）；
 * frameType='line' 时同原版 type='line' 门控：隐藏圆角与内边距（含拆分形态），其余项保留。
 */
export function FrameSettingsField({
    data,
    onChange,
    field = "frame",
    label,
    variant,
    frameType,
    splitInnerPadding = false,
    hideBg = false,
    hidePadding = false,
    hideBoxRadius = false,
    hideBoxPadding = false,
    hideHead = false
}: FieldProps & {
    field?: string;
    label?: string;
    variant?: "notice" | "searchBar";
    frameType?: string;
    splitInnerPadding?: boolean;
    /** 原版 CommonFrameEdit 的 hide-bg / hide-padding / hide-box-radius / hide-box-padding 门控 */
    hideBg?: boolean;
    hidePadding?: boolean;
    hideBoxRadius?: boolean;
    hideBoxPadding?: boolean;
    /** 原版 CommonFrameEdit type='live' 时不渲染分隔线与「模块设置」标题 */
    hideHead?: boolean;
}) {
    const frame = data?.[field] ?? {};
    const set = (patch: Patch) => onChange({ [field]: { ...frame, ...patch } });
    const sub: FieldProps = { data: frame, onChange: (patch: Patch) => set(patch) };
    // 原版 CommonFrameEdit type='line'：圆角与内边距（单项/拆分两形态）均不渲染
    const isLine = frameType === "line";

    if (variant === "notice" || variant === "searchBar") {
        return (
            <>
                <ColorField {...sub} field="textColor" label="文字颜色" resetValue={defaultFrame.textColor} />
                <ColorField {...sub} field="itemBackgroundColor" label="组件背景" resetValue={defaultFrame.itemBackgroundColor} />
                <RangeField {...sub} field="itemRadius" label="组件圆角" max={50} />
                <RangeField {...sub} field="itemHeight" label="组件高度" max={50} />
                <ColorField {...sub} field="backgroundColor" label="背景颜色" resetValue={defaultFrame.backgroundColor} />
                <RangeField {...sub} field="boxPadding" label="左右边距" max={30} />
                <RangeField {...sub} field="boxPaddingTop" label="上边距" max={30} />
                <RangeField {...sub} field="boxPaddingBottom" label="下边距" max={30} />
            </>
        );
    }

    return (
        <>
            {hideHead ? null : <DecDivider />}
            {hideHead ? null : <DecSpreadTitle title={label ?? "模块设置"} />}
            {hideBg ? null : <ColorField {...sub} field="backgroundColor" label="背景颜色" resetValue={defaultFrame.backgroundColor} />}
            {hideBoxRadius || isLine ? null : <RangeField {...sub} field="boxRadius" label="圆角" max={30} />}
            {hidePadding || isLine ? null : splitInnerPadding ? (
                <>
                    <RangeField {...sub} field="innerPaddingLeftRight" label="左右内边距" max={30} />
                    <RangeField {...sub} field="innerPaddingTopBottom" label="上下内边距" max={30} />
                </>
            ) : (
                <RangeField {...sub} field="innerPadding" label="内边距" max={30} />
            )}
            {frameType === "titleAd" ? <RangeField {...sub} field="leftAndRightPadding" label="左右内边距" max={30} /> : null}
            {hideBoxPadding ? null : (
                <>
                    <RangeField {...sub} field="boxPadding" label="左右边距" max={30} />
                    <RangeField {...sub} field="boxPaddingTop" label="上边距" max={30} />
                    <RangeField {...sub} field="boxPaddingBottom" label="下边距" max={30} />
                </>
            )}
        </>
    );
}

// ---------------- 颜色对列表（colorA~colorB，tagNav 标签配色等） ----------------

export function ColorPairListField({
    data,
    onChange,
    field,
    label,
    defaults,
}: FieldProps & { field: string; label: string; defaults: Array<{ colorA: string; colorB: string }> }) {
    const list: Array<{ colorA?: string; colorB?: string }> = data?.[field] || [];
    const setItem = (index: number, key: "colorA" | "colorB", value: string) => {
        const next = list.map((item, i) => (i === index ? { ...item, [key]: value } : item));
        onChange({ [field]: next });
    };
    const addOne = () => onChange({ [field]: [...list, { ...defaults[list.length % defaults.length] }] });
    const removeOne = (index: number) => onChange({ [field]: list.filter((_, i) => i !== index) });
    return (
        <SettingGroup title={label}>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {list.map((item, index) => (
                    <div key={index} style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <span className="dec-color-trigger">
                            <AlphaColorInput value={item.colorA || "#cccccc"} onChange={(v) => setItem(index, "colorA", v)} />
                        </span>
                        <span style={{ color: "#98a2b3", fontSize: 12 }}>→</span>
                        <span className="dec-color-trigger">
                            <AlphaColorInput value={item.colorB || item.colorA || "#cccccc"} onChange={(v) => setItem(index, "colorB", v)} />
                        </span>
                        <button type="button" onClick={() => removeOne(index)} style={{ marginLeft: "auto", fontSize: 12, color: "var(--semi-color-danger)", background: "transparent", border: 0, cursor: "pointer" }}>
                            删除
                        </button>
                    </div>
                ))}
                <AddItemButton onClick={addOne} style={{ marginTop: 0 }}>
                    添加配色
                </AddItemButton>
            </div>
        </SettingGroup>
    );
}
