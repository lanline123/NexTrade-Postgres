/**
 * 模块设置面板通用控件（Semi/React 版，壳层对齐 view/admin 的 dec-* 体系）
 *
 * 结构/数值口径（原版 src/css/decorate.less + 各共享 Edit 组件）：
 * - 字段行 `.dec-edit-group`（padding 10px 16px；标签 .label 14px #969799 margin-right 16px；控件区 .dec-edit-group-con flex:1 右对齐）
 * - 分组标题三形态：dec-spread-title（平面粗体）/ dec-edit-title-box（灰带 + 3px 蓝条）/ dec-edit-section-title
 * - 单选组 `.dec-radio-group`：32px 分段按钮、border #ebedf0、选中主题色 + --tig-item-active-bg 近似底色
 * - 滑杆 `.dec-range-group`：range + 数字框（无步进钮）
 * - 颜色行 `.dec-color-group`：「重置」文字链 + 方形色块触发器（弹层为 AlphaColorInput，预置色为原版 SelectColor 14 色）
 * - 图片列表 `.dec-pic-group`：60×60 contain 图盒 + 底部 20px「更换图片」条 + hover 删除徽标 + 信息行 + 整宽添加按钮 + 上移/下移
 *
 * 统一 onChange(patch) 协议：所有控件只上报字段级 patch，由 store 浅合并进模块数据。
 */
import { Input, Select, Switch, Upload, Toast } from "@douyinfe/semi-ui";
import { IconPlus } from "@douyinfe/semi-icons";
import AlphaColorInput from "../../../components/AlphaColorInput";
import { imageFormat, pickPic } from "./format";
import { defaultTitle, defaultFrame } from "../contract/sharedDefaults";

import { useI18n } from '../../../i18n';
type Patch = Record<string, any>;
interface FieldProps {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

// ---------------- 布局 ----------------

/** 分组标题形态：box=dec-edit-title-box（默认）/ spread=dec-spread-title / section=dec-edit-section-title */
export type SettingGroupVariant = "box" | "spread" | "section";

export function SettingGroup({ title, children, variant = "box" }: { title?: string; children: React.ReactNode; variant?: SettingGroupVariant }) {
    const { t } = useI18n();
    return (
        <div className="h5-editor-setting-group">
            {title ? (
                variant === "spread" ? (
                    <div className="dec-spread-title">
                        <div className="title">{t(title)}</div>
                    </div>
                ) : variant === "section" ? (
                    <div className="dec-edit-section-title">{t(title)}</div>
                ) : (
                    <div className="dec-edit-title-box">
                        <div className="title">{t(title)}</div>
                    </div>
                )
            ) : null}
            {children}
        </div>
    );
}

/** 平面分组标题（原版 .dec-spread-title） */
export function DecSpreadTitle({ title }: { title: string }) {
    const { t } = useI18n();
    return (
        <div className="dec-spread-title">
            <div className="title">{t(title)}</div>
        </div>
    );
}

/** 分隔线（原版 .dec-divider-line） */
export function DecDivider() {
    return <div className="dec-divider-line" />;
}

/** 字段行（原版 .dec-edit-group）：左标签 + 右控件区 */
export function SettingRow({ label, children, block = false }: { label: string; children: React.ReactNode; block?: boolean }) {
    const { t } = useI18n();
    return (
        <div className={`dec-edit-group${block ? " dec-edit-group-block" : ""}`}>
            <div className="dec-edit-group-title">
                <div className="label">{label ? t(label) : ""}</div>
            </div>
            <div className="dec-edit-group-con">{children}</div>
        </div>
    );
}

// ---------------- 单选组（原版 .dec-radio-group 分段样式） ----------------

interface RadioOption {
    value: number | string;
    label: string;
}

/** 裸分段单选组（无字段行外壳，供 dec-edit-group-block 等容器内嵌使用） */
export function SegmentedRadio({ value, options, onChange }: { value: any; options: RadioOption[]; onChange: (v: number | string) => void }) {
    const { t } = useI18n();
    return (
        <div className="dec-radio-group" style={{ flex: 1, minWidth: 0 }}>
            {options.map((o) => (
                <label
                    key={String(o.value)}
                    className={`el-radio-button${String(value) === String(o.value) ? " is-active" : ""}`}
                    onClick={() => onChange(o.value)}
                >
                    <span className="el-radio-button__inner">{t(o.label)}</span>
                </label>
            ))}
        </div>
    );
}

/** 单选（无标签行，整行分段组） */
export function RadioField({ data, onChange, field, options }: FieldProps & { field: string; options: RadioOption[] }) {
    return (
        <div className="dec-edit-group">
            <div className="dec-edit-group-con">
                <SegmentedRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v })} />
            </div>
        </div>
    );
}

/** 带标题的单选 */
export function RadioRowField({ data, onChange, field, label, options }: FieldProps & { field: string; label: string; options: RadioOption[] }) {
    return (
        <SettingRow label={label}>
            <SegmentedRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v })} />
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
    const clamp = (v: number) => Math.max(min, Math.min(max, Number.isFinite(v) ? v : 0));
    return (
        <SettingRow label={label}>
            <div className="dec-range-group">
                <div className="dec-range-slider-wrap">
                    <input
                        type="range"
                        className="dec-range-slider-native"
                        min={min}
                        max={max}
                        step={step}
                        value={clamp(value)}
                        onChange={(e) => onChange({ [field]: clamp(Number(e.target.value)) })}
                    />
                </div>
                <div className="dec-range-input">
                    <input
                        type="number"
                        className="dec-range-number"
                        min={min}
                        max={max}
                        step={step}
                        value={value}
                        onChange={(e) => onChange({ [field]: clamp(Number(e.target.value)) })}
                        onBlur={(e) => onChange({ [field]: clamp(Number(e.target.value)) })}
                    />
                </div>
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
    const clamp = (v: number) => Math.max(min, Math.min(max, Number.isFinite(v) ? v : 0));
    return (
        <SettingRow label={label}>
            <div className="dec-range-group">
                <div className="dec-range-slider-wrap">
                    <input
                        type="range"
                        className="dec-range-slider-native"
                        min={min}
                        max={max}
                        step={step}
                        value={clamp(value)}
                        onChange={(e) => onChange({ [field]: clamp(Number(e.target.value)) })}
                    />
                </div>
                <div className="dec-range-input">
                    <input
                        type="number"
                        className="dec-range-number"
                        min={min}
                        max={max}
                        step={step}
                        value={value}
                        onChange={(e) => onChange({ [field]: clamp(Number(e.target.value)) })}
                        onBlur={(e) => onChange({ [field]: clamp(Number(e.target.value)) })}
                    />
                    {suffix ? <span className="dec-range-suffix">{suffix}</span> : null}
                </div>
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
    const { t } = useI18n();
    return (
        <SettingRow label={label}>
            <div className="dec-color-group">
                <div className="dec-color-button">
                    {allowEmpty ? (
                        <a className="dec-color-reset" onClick={() => onChange({ [field]: resetValue })}>
                            {t('重置')}
                        </a>
                    ) : null}
                    <span className="dec-color-trigger">
                        <AlphaColorInput value={String(data?.[field] ?? "")} onChange={(v) => onChange({ [field]: v })} />
                    </span>
                </div>
            </div>
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

const UPLOAD_ACTION = "/adminapi/file/uploadImage";

function useUploadUrl(t: (key: string) => string) {
    return (body: any): string => {
        const url = body?.data?.url || body?.url;
        if (!url) Toast.error(t("上传失败"));
        return url || "";
    };
}

/** 单图选择（原版 PicSelect：60×60 图盒 + 底部「更换图片」条 + 下方重置位） */
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
    const urlOf = useUploadUrl(t);
    const url = pic ? pickPic(pic) : "";
    return (
        <div className="dec-pic-select">
            <Upload
                action={UPLOAD_ACTION}
                accept="image/*"
                showUploadList={false}
                onSuccess={(body: any) => {
                    const u = urlOf(body);
                    if (u) onSelect({ picUrl: u, picThumb: u });
                }}
                onError={() => Toast.error(t("上传失败"))}
            >
                <div className="item-image">
                    <div className="item-image-con">
                        {url ? (
                            <>
                                <img src={imageFormat(url)} alt="" />
                                <span className="change-image">{t('更换图片')}</span>
                            </>
                        ) : (
                            <span className="item-image-empty">
                                <IconPlus size="large" />
                            </span>
                        )}
                    </div>
                </div>
            </Upload>
            {onReset ? (
                <a className="dec-pic-reset" onClick={onReset}>
                    {t('重置')}
                </a>
            ) : null}
        </div>
    );
}

/** 单图字段行（标签 + PicSelect 图盒） */
export function PicSelectField({ data, onChange, field, label }: FieldProps & { field: string; label: string }) {
    const pic = data?.[field];
    const picObj = pic && typeof pic === "object" ? pic : typeof pic === "string" && pic ? { picUrl: pic, picThumb: pic } : null;
    return (
        <SettingRow label={label}>
            <PicSelectBox pic={picObj} onSelect={(p) => onChange({ [field]: p })} onReset={() => onChange({ [field]: { picUrl: "", picThumb: "" } })} />
        </SettingRow>
    );
}

/** 单图编辑器（块级：标签行 + 图盒，原 PicSelect 形态） */
export function SinglePicField({ data, onChange, field, label }: FieldProps & { field: string; label: string }) {
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
                            <PicSelectBox pic={picObj} onSelect={(p) => onChange({ [field]: p })} onReset={() => onChange({ [field]: { picUrl: "", picThumb: "" } })} />
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
 * 图片列表编辑器（原版 PicList 形态）：
 * 每项 = 60×60 contain 图盒（底部 20px「更换图片」条）+ 信息行（标题/描述/链接）+ hover 删除徽标 + 上移/下移；
 * 底部整宽 `.dec-pic-add-btn` 添加按钮。showText/showDesc/showLink 默认同原版（true/false/true）。
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
    addText = "添加图片"
}: FieldProps & { field: string; label: string; max?: number; showText?: boolean; showDesc?: boolean; showLink?: boolean; addText?: string }) {
    const { t } = useI18n();
    const urlOf = useUploadUrl(t);
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
                                <div className="item-image">
                                    <Upload
                                        action={UPLOAD_ACTION}
                                        accept="image/*"
                                        showUploadList={false}
                                        onSuccess={(body: any) => {
                                            const u = urlOf(body);
                                            if (u) patchItem(index, { picUrl: u, picThumb: u });
                                        }}
                                        onError={() => Toast.error(t("上传失败"))}
                                    >
                                        <div className="item-image-con">
                                            {pickPic(item) ? <img src={imageFormat(pickPic(item))} alt="" /> : null}
                                            <span className="change-image">{t('更换图片')}</span>
                                        </div>
                                    </Upload>
                                </div>
                                <div className="item-info">
                                    {showText ? (
                                        <div className="item-info-item">
                                            <span className="lable">{t('标题')}</span>
                                            <Input
                                                size="small"
                                                placeholder={t('建议10个字以内，可不填')}
                                                value={item.picTitle ?? ""}
                                                onChange={(v: string) => patchItem(index, { picTitle: v })}
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
                                                onChange={(v: string) => patchItem(index, { picDesc: v })}
                                            />
                                        </div>
                                    ) : null}
                                    {showLink ? (
                                        <div className="item-info-item">
                                            <span className="lable">{t('链接')}</span>
                                            <Input
                                                size="small"
                                                placeholder={t('跳转链接，如 /pages/product/index?id=1')}
                                                value={item.picLink?.link ?? ""}
                                                onChange={(v: string) =>
                                                    patchItem(index, { picLink: { ...(item.picLink || { title: "" }), link: v } })
                                                }
                                            />
                                        </div>
                                    ) : null}
                                    <div className="item-info-item dec-pic-sort-row">
                                        <a className={index === 0 ? "is-disabled" : ""} onClick={() => moveItem(index, -1)}>
                                            {t('上移')}
                                        </a>
                                        <a className={index === list.length - 1 ? "is-disabled" : ""} onClick={() => moveItem(index, 1)}>
                                            {t('下移')}
                                        </a>
                                    </div>
                                </div>
                                <span className="del-item" title={t('删除')} onClick={() => setList(list.filter((_, i) => i !== index))}>
                                    ×
                                </span>
                            </div>
                        </div>
                    ))}
                    {list.length < max && (
                        <Upload
                            action={UPLOAD_ACTION}
                            accept="image/*"
                            showUploadList={false}
                            onSuccess={(body: any) => {
                                const u = urlOf(body);
                                if (u) setList([...list, { picUrl: u, picThumb: u, picTitle: "", picDesc: "", picLink: { link: "", title: "" } }]);
                            }}
                            onError={() => Toast.error(t("上传失败"))}
                        >
                            <a className="dec-pic-add-btn">
                                <IconPlus style={{ position: "absolute", left: 15 }} />
                                {t(addText)}
                            </a>
                        </Upload>
                    )}
                </div>
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
                                    <Input
                                        value={moreLinkObj.link ?? ""}
                                        placeholder={t('跳转链接，如 /pages/product/index?id=1')}
                                        style={{ width: 240, maxWidth: "100%" }}
                                        onChange={(v: string) => set({ moreLink: { ...moreLinkObj, link: v } })}
                                    />
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
                        <Input
                            value={moreLinkObj.link ?? ""}
                            placeholder={t('跳转链接，如 /pages/product/index?id=1')}
                            style={{ width: 240, maxWidth: "100%" }}
                            onChange={(v: string) => set({ moreLink: { ...moreLinkObj, link: v } })}
                        />
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
 * + 背景颜色/左右边距/上边距/下边距，且无分隔线与标题（同原版 type 门控）。
 */
export function FrameSettingsField({
    data,
    onChange,
    field = "frame",
    label,
    variant,
    frameType,
    splitInnerPadding = false
}: FieldProps & {
    field?: string;
    label?: string;
    variant?: "notice" | "searchBar";
    frameType?: string;
    splitInnerPadding?: boolean;
}) {
    const frame = data?.[field] ?? {};
    const set = (patch: Patch) => onChange({ [field]: { ...frame, ...patch } });
    const sub: FieldProps = { data: frame, onChange: (patch: Patch) => set(patch) };

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
            <DecDivider />
            <DecSpreadTitle title={label ?? "模块设置"} />
            <ColorField {...sub} field="backgroundColor" label="背景颜色" resetValue={defaultFrame.backgroundColor} />
            <RangeField {...sub} field="boxRadius" label="圆角" max={30} />
            {splitInnerPadding ? (
                <>
                    <RangeField {...sub} field="innerPaddingLeftRight" label="左右内边距" max={30} />
                    <RangeField {...sub} field="innerPaddingTopBottom" label="上下内边距" max={30} />
                </>
            ) : (
                <RangeField {...sub} field="innerPadding" label="内边距" max={30} />
            )}
            {frameType === "titleAd" ? <RangeField {...sub} field="leftAndRightPadding" label="左右内边距" max={30} /> : null}
            <RangeField {...sub} field="boxPadding" label="左右边距" max={30} />
            <RangeField {...sub} field="boxPaddingTop" label="上边距" max={30} />
            <RangeField {...sub} field="boxPaddingBottom" label="下边距" max={30} />
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
                <button type="button" onClick={addOne} style={{ height: 26, fontSize: 12, border: "1px solid var(--semi-color-border)", borderRadius: 4, background: "transparent", cursor: "pointer" }}>
                    + 添加配色
                </button>
            </div>
        </SettingGroup>
    );
}
