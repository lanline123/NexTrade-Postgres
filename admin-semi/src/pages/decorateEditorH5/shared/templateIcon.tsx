/**
* 视觉模板选择器（共享）
*
* 原版契约（ref-editor view/admin editor/modules 各 Settings.vue 及
* src/modules 各 Edit.vue）：「选择模板 / 列表样式 / 指示器样式 / 图片填充 /
* 对齐 / 字号 / 粗细」等字段为图标 radio——el-radio-button 内只有
* <i class="ico-decorate icon-dec-*"> 或 <i class="iconfont-admin icon-*">
* 字形，文字只出现在 el-tooltip 与标题行右侧.value（当前选中 label）。
* 我方此前一律以文字圆点/分段 radio 承接，属视觉选择器降级（用户
* 2026-11-20 在页面 10106 图片广告/图片魔方上点名）。
*
* 改口径说明：图形载体不用 tigshop 字体图标（全站已弃 tig iconfont，
* 2026-11-15 用户裁定），按 icon-dec-* / iconfont-admin 语义以 inline SVG
* 示意图（currentColor，选中态随胶囊字色反转）或同尺寸字形字母（字号/
* 粗细的「A」字形，与原版 icon-zitiyanse / icon-bianji-ziti 以字号和
* 粗细区分选项的口径一致）呈现；数据契约（字段名/数值/默认值）零变化。
* 形态与既有 IconStyleRadioField 先例一致：Semi RadioGroup type="button"
* 分段，同挂专属类 dec-icon-radio——editor.less 专属规则令其按内容紧凑宽
* 排布（2026-11-21 用户点名过宽后定案；此前无专属类、被通用均分规则整宽
* 拉伸，两选项时每项各占控件区一半）。原版 .dec-radio-group 同为紧凑宽
* （组无 width:100%，靠右于 .dec-edit-group-con，组内 flex:1 仅在紧凑宽
* 内均分，原版 decorate.less:933-950）。
*/
import { Radio, RadioGroup, Tooltip} from "@douyinfe/semi-ui";
import { SettingRow} from "./settingWidgets";
import { useI18n} from "../../../i18n";

type Patch = Record<string, any>;
interface FieldProps {
data: Record<string, any>;
onChange: (patch: Patch) => void;
}

export interface TemplateIconOption {
value: number | string;
label: string;
/** 示意图名（见 TemplatePictogram 的图谱）；与 aSize/aBold 二选一 */
icon?: string;
/** 字形字母选项（字号/粗细）：以该字号渲染字母 A */
aSize?: number;
/** 字形字母选项：是否加粗渲染 */
aBold?: boolean;
}

function matchOption(value: any, options: TemplateIconOption[]): TemplateIconOption | undefined {
return options.find((o) => String(o.value) === String(value));
}

function changeValue(raw: any, options: TemplateIconOption[]): number | string {
const matched = matchOption(raw, options);
return matched? matched.value: raw;
}

/**
* icon-dec-* / iconfont-admin 语义图谱（inline SVG，viewBox 32×24）。
* 命名与原版图标类一一对应：up2end/carousel/big-slide/small-slide/
* cuberow（一行两个）/cuberow2（一行三个）/cuberow1（一行四个）/cube（2×2）/
* cubeto（左一右二）/cubeto1（左一右三）/cube-upto（上一下二）/hybrid（一大两小）/
* hybrid5（两行五个）/list（详细列表）/swipe（横向滑动）/indicator-1/2/3/
* fill-cover/fill-contain/align-left/align-center/align-right/mode-slide/mode-slide2。
* small/three 为 product 系对 cuberow/cuberow2 的别名类名，图谱内归一。
*/
export function TemplatePictogram({ icon}: { icon: string}) {
const name = icon === "small"? "cuberow": icon === "three"? "cuberow2": icon;
const rects = (() => {
switch (name) {
case "up2end":
return <rect x={3} y={4} width={26} height={16} rx={1.5} />;
case "carousel":
return (
<>
<rect x={3} y={3} width={26} height={13} rx={1.5} />
<circle cx={11} cy={20.5} r={1.7} />
<circle cx={16} cy={20.5} r={1.7} fillOpacity={0.4} />
<circle cx={21} cy={20.5} r={1.7} fillOpacity={0.4} />
</>
);
case "big-slide":
return (
<>
<rect x={2} y={4} width={20} height={16} rx={1.5} />
<rect x={24} y={4} width={6} height={16} rx={1.5} fillOpacity={0.4} />
</>
);
case "small-slide":
return (
<>
<rect x={2} y={4} width={11} height={16} rx={1.5} />
<rect x={15} y={4} width={11} height={16} rx={1.5} fillOpacity={0.55} />
<rect x={28} y={4} width={2.5} height={16} rx={1.2} fillOpacity={0.3} />
</>
);
case "cuberow":
return (
<>
<rect x={2} y={4} width={13} height={16} rx={1.5} />
<rect x={17} y={4} width={13} height={16} rx={1.5} />
</>
);
case "cuberow2":
return (
<>
<rect x={2} y={4} width={8.6} height={16} rx={1.5} />
<rect x={11.7} y={4} width={8.6} height={16} rx={1.5} />
<rect x={21.4} y={4} width={8.6} height={16} rx={1.5} />
</>
);
case "cuberow1":
return (
<>
<rect x={2} y={4} width={6} height={16} rx={1.2} />
<rect x={9.3} y={4} width={6} height={16} rx={1.2} />
<rect x={16.6} y={4} width={6} height={16} rx={1.2} />
<rect x={24} y={4} width={6} height={16} rx={1.2} />
</>
);
case "cube":
return (
<>
<rect x={2} y={4} width={13} height={7} rx={1.2} />
<rect x={17} y={4} width={13} height={7} rx={1.2} />
<rect x={2} y={13} width={13} height={7} rx={1.2} />
<rect x={17} y={13} width={13} height={7} rx={1.2} />
</>
);
case "cubeto":
return (
<>
<rect x={2} y={4} width={13} height={16} rx={1.5} />
<rect x={17} y={4} width={13} height={7} rx={1.2} />
<rect x={17} y={13} width={13} height={7} rx={1.2} />
</>
);
case "cubeto1":
return (
<>
<rect x={2} y={4} width={13} height={16} rx={1.5} />
<rect x={17} y={4} width={13} height={4.6} rx={1.2} />
<rect x={17} y={9.7} width={13} height={4.6} rx={1.2} />
<rect x={17} y={15.4} width={13} height={4.6} rx={1.2} />
</>
);
case "cube-upto":
return (
<>
<rect x={2} y={4} width={28} height={7} rx={1.2} />
<rect x={2} y={13} width={13} height={7} rx={1.2} />
<rect x={17} y={13} width={13} height={7} rx={1.2} />
</>
);
case "hybrid":
return (
<>
<rect x={2} y={4} width={16} height={16} rx={1.5} />
<rect x={20} y={4} width={10} height={7} rx={1.2} />
<rect x={20} y={13} width={10} height={7} rx={1.2} />
</>
);
case "hybrid5":
return (
<>
<rect x={2} y={4} width={13} height={7} rx={1.2} />
<rect x={17} y={4} width={13} height={7} rx={1.2} />
<rect x={2} y={13} width={8.6} height={7} rx={1.2} />
<rect x={11.7} y={13} width={8.6} height={7} rx={1.2} />
<rect x={21.4} y={13} width={8.6} height={7} rx={1.2} />
</>
);
case "list":
return (
<>
<rect x={2} y={4} width={10} height={16} rx={1.5} />
<rect x={14} y={5.5} width={16} height={2.6} rx={1.3} />
<rect x={14} y={10.7} width={16} height={2.6} rx={1.3} fillOpacity={0.55} />
<rect x={14} y={15.9} width={16} height={2.6} rx={1.3} fillOpacity={0.35} />
</>
);
case "swipe":
return (
<>
<rect x={2} y={4} width={17} height={16} rx={1.5} />
<rect x={21} y={4} width={5} height={16} rx={1.5} fillOpacity={0.4} />
<polygon points="27.5,8.5 30.5,12 27.5,15.5" />
</>
);
case "indicator-1":
return (
<>
<circle cx={8} cy={12} r={2.2} fillOpacity={0.35} />
<circle cx={16} cy={12} r={2.2} />
<circle cx={24} cy={12} r={2.2} fillOpacity={0.35} />
</>
);
case "indicator-2":
return (
<>
<rect x={3} y={10.7} width={7} height={2.6} rx={1.3} fillOpacity={0.35} />
<rect x={12.5} y={10.7} width={7} height={2.6} rx={1.3} />
<rect x={22} y={10.7} width={7} height={2.6} rx={1.3} fillOpacity={0.35} />
</>
);
case "indicator-3":
return (
<>
<rect x={3} y={6} width={26} height={12} rx={6} fill="none" stroke="currentColor" strokeWidth={1.6} />
<circle cx={11} cy={12} r={1.8} fillOpacity={0.4} />
<circle cx={16} cy={12} r={1.8} />
<circle cx={21} cy={12} r={1.8} fillOpacity={0.4} />
</>
);
case "fill-cover":
return (
<>
<rect x={3} y={4} width={26} height={16} rx={1.5} fillOpacity={0.85} />
<rect x={3} y={4} width={26} height={16} rx={1.5} fill="none" stroke="currentColor" strokeWidth={1.4} />
</>
);
case "fill-contain":
return (
<>
<rect x={3} y={4} width={26} height={16} rx={1.5} fill="none" stroke="currentColor" strokeWidth={1.4} />
<rect x={10} y={8} width={12} height={8} rx={1} />
</>
);
case "align-left":
return (
<>
<rect x={3} y={5} width={18} height={2.6} rx={1.3} />
<rect x={3} y={10.7} width={12} height={2.6} rx={1.3} />
<rect x={3} y={16.4} width={15} height={2.6} rx={1.3} />
</>
);
case "align-center":
return (
<>
<rect x={7} y={5} width={18} height={2.6} rx={1.3} />
<rect x={10} y={10.7} width={12} height={2.6} rx={1.3} />
<rect x={8.5} y={16.4} width={15} height={2.6} rx={1.3} />
</>
);
case "align-right":
return (
<>
<rect x={11} y={5} width={18} height={2.6} rx={1.3} />
<rect x={17} y={10.7} width={12} height={2.6} rx={1.3} />
<rect x={14} y={16.4} width={15} height={2.6} rx={1.3} />
</>
);
case "effect-default":
return (
<>
<rect x={4} y={3} width={24} height={18} rx={1.5} fill="none" stroke="currentColor" strokeWidth={1.5} />
<rect x={7} y={6} width={18} height={4} rx={2} />
<rect x={7} y={13} width={18} height={5} rx={1} fillOpacity={0.4} />
</>
);
case "effect-ceiling":
return (
<>
<rect x={4} y={3} width={24} height={18} rx={1.5} fill="none" stroke="currentColor" strokeWidth={1.5} />
<rect x={4} y={3} width={24} height={5} rx={1.5} />
<polygon points="16,11 19.5,16.5 12.5,16.5" />
</>
);
case "mode-fixed":
return (
<>
<rect x={3} y={5} width={8} height={6} rx={1.2} />
<rect x={12.5} y={5} width={8} height={6} rx={1.2} />
<rect x={22} y={5} width={8} height={6} rx={1.2} />
<rect x={3} y={13.5} width={8} height={6} rx={1.2} />
<rect x={12.5} y={13.5} width={8} height={6} rx={1.2} />
<rect x={22} y={13.5} width={8} height={6} rx={1.2} />
</>
);
case "mode-slide":
return (
<>
<rect x={2} y={7} width={7} height={10} rx={1.2} />
<rect x={11} y={7} width={7} height={10} rx={1.2} />
<rect x={20} y={7} width={7} height={10} rx={1.2} fillOpacity={0.55} />
<polygon points="28,9 31,12 28,15" />
</>
);
case "mode-slide2":
return (
<>
<rect x={2} y={4} width={8.6} height={6} rx={1.2} />
<rect x={11.7} y={4} width={8.6} height={6} rx={1.2} />
<rect x={21.4} y={4} width={8.6} height={6} rx={1.2} />
<rect x={2} y={14} width={8.6} height={6} rx={1.2} />
<rect x={11.7} y={14} width={8.6} height={6} rx={1.2} />
<rect x={21.4} y={14} width={8.6} height={6} rx={1.2} />
</>
);
default:
return <rect x={3} y={4} width={26} height={16} rx={1.5} />;
}
})();
return (
<svg width={26} height={20} viewBox="0 0 32 24" fill="currentColor" aria-hidden="true" style={{ display: "block"}}>
{rects}
</svg>
);
}

function OptionGlyph({ option}: { option: TemplateIconOption}) {
if (option.aSize!== undefined || option.aBold!== undefined) {
return (
<span style={{ fontSize: option.aSize?? 14, fontWeight: option.aBold? 700: 400, lineHeight: 1, display: "inline-block"}}>A</span>
);
}
return <TemplatePictogram icon={option.icon?? ""} />;
}

/** 裸视觉单选组（Semi RadioGroup type="button"，dec-icon-radio 紧凑宽；悬停 Tooltip 显示文字 label，另带 title 兜底） */
export function TemplateIconRadio({
value,
options,
onChange
}: {
value: any;
options: TemplateIconOption[];
onChange: (v: number | string) => void;
}) {
const { t} = useI18n();
const matched = matchOption(value, options);
return (
<RadioGroup
type="button"
className="dec-icon-radio"
value={matched? matched.value: value}
onChange={(e: any) => onChange(changeValue(e?.target?.value, options))}
>
{options.map((o) => (
<Radio key={String(o.value)} value={o.value}>
<Tooltip content={t(o.label)} position="bottom" showArrow={false}>
<span title={t(o.label)} style={{ display: "inline-flex", alignItems: "center", justifyContent: "center", minWidth: 26, minHeight: 20}}>
<OptionGlyph option={o} />
</span>
</Tooltip>
</Radio>
))}
</RadioGroup>
);
}

/**
* 块级视觉选择器（原版 dec-edit-group-block 结构：标题行 label + 右侧
*.value 显示当前选中 label，控件区整宽横排在下方）。
*/
export function TemplateIconRadioField({
data,
onChange,
field,
label,
options
}: FieldProps & { field: string; label: string; options: TemplateIconOption[]}) {
const { t} = useI18n();
const matched = matchOption(data?.[field], options);
return (
<div className="dec-edit-group dec-edit-group-block">
<div className="dec-edit-group-title">
<div className="label">{t(label)}</div>
<div className="value">{matched? t(matched.label): ""}</div>
</div>
<div className="dec-edit-group-con">
<TemplateIconRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v})} />
</div>
</div>
);
}

/** 行级视觉选择器（左标签 + 右控件区，同 RadioRowField 布局，控件换图形钮） */
export function TemplateIconRadioRowField({
data,
onChange,
field,
label,
options
}: FieldProps & { field: string; label: string; options: TemplateIconOption[]}) {
return (
<SettingRow label={label}>
<TemplateIconRadio value={data?.[field]} options={options} onChange={(v) => onChange({ [field]: v})} />
</SettingRow>
);
}
