/**
* 视频号 设置（对齐 view/admin videoNumber/Settings.vue）
*
* 顶部说明 tip + 原版自有下划线页签（内容设置/样式设置）。
* 内容：内容类型（video 视频/live 直播）→ 视频号ID 下拉（选项取既有接口 /content/wechatVideoAccount/list
* 的 finderId，同原版 getWechatVideoAccountList 落地）+ 同主体提示（原版「去添加」跳路由页，
* 本后台为单页应用无对应路由，未挂链接，见摘要）；video 时视频ID 输入 + 视频号助手提示
* （「去获取」外链 https://channels.weixin.qq.com/platform 同原版）；live 时标题（maxlength 14 带字数）、
* 标题颜色（default/theme/custom 三段 + custom 时显示颜色，重置 #2A2A2A）、商品图片 PicList（max 3，
* 原版为图库多选+拖拽列表，本实现以 dec PicListField 承接同结构 pic 对象数组）。
* 样式：CommonFrameEdit type="live"（hideHead 承接）。
* 写回行为对齐原版：打开设置时以原版字面量 mergeDefaultModule 烧录缺失字段（liveTitle 空 →「这场直播特别有料」等）；
* 视频号选项加载后，videoNumberId 为空自动选首个写回、liveTitle 为空以当前视频号标题（缺省为默认标题）回填；
* 切换到 live 且 liveTitle 为空、手动改选视频号且 liveTitle 为空时同样回填；liveTitle 超 14 字截断、
* productImages 规整（上限 3 + 补 picKey）等 watch 写回一并复刻。写回均带 diff/已执行守卫，避免重复写与循环。
*/
import { useEffect, useRef, useState} from "react";
import { Input, Select} from "@douyinfe/semi-ui";
import { apiGet} from "../../../../../api";
import { cloneDeep, defaultFrame, mergeDefaultModule } from "../../../contract/sharedDefaults";
import { SettingGroup, SettingRow, SegmentedRadio, ColorField, PicListField, FrameSettingsField, RadioRowField } from "../../../shared/settingWidgets";

const TIP = "视频号组件为视频号配套组件：通过该组件可以实现自定义进入视频号视频或直播，帮助商户提升视频号相关内容的曝光率，提升视频号热度。";

const DEFAULT_LIVE_TITLE = "这场直播特别有料";
/** 原版 Settings setup 的 mergeDefaultModule 字面量（与注册默认值不同：liveTitle 为默认标题、frame 仅三项覆盖） */
const SETTINGS_BURN_DEFAULTS: Record<string, any> = {
titleText: "视频号",
contentType: "video",
videoNumberId: "",
videoId: "",
liveTitle: DEFAULT_LIVE_TITLE,
titleColorType: "default",
titleColor: "#2A2A2A",
productImages: [] as any[],
frame: { ...cloneDeep(defaultFrame), backgroundColor: "#ffffff", innerPadding: 12, boxRadius: 8 }
};

export default function VideoNumberSettings({ data, onChange, moduleIndex}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void; moduleIndex?: number}) {
const [tab, setTab] = useState<"content" | "style">("content");
const contentType = data?.contentType?? "video";
const [options, setOptions] = useState<any[]>([]);
const dataRef = useRef(data);
dataRef.current = data;
const onChangeRef = useRef(onChange);
onChangeRef.current = onChange;
const openedKeyRef = useRef<string | null>(null);
const optionsWriteBackDoneRef = useRef(false);

// 打开即烧录（同原版 setup 的 mergeDefaultModule）：每个模块仅执行一次，diff 后单次写回
useEffect(() => {
const key = moduleIndex !== undefined && moduleIndex !== null? `idx:${moduleIndex}`: "single";
if (openedKeyRef.current === key) return;
openedKeyRef.current = key;
const current = dataRef.current?? {};
const merged = cloneDeep(current);
mergeDefaultModule(merged, cloneDeep(SETTINGS_BURN_DEFAULTS));
const patch: Record<string, any> = {};
Object.keys(SETTINGS_BURN_DEFAULTS).forEach((k) => {
if (JSON.stringify(merged[k]) !== JSON.stringify(current[k])) patch[k] = merged[k];
});
if (Object.keys(patch).length) onChangeRef.current(patch);
});

useEffect(() => {
let alive = true;
apiGet<any>("/content/wechatVideoAccount/list", { page: 1, size: 100, keyword: ""})
.then((res: any) => {
if (alive) setOptions(res?.records || res?.list || []);
})
.catch(() => undefined);
return () => {
alive = false;
};
}, []);

// 选项加载后写回（同原版 loadVideoNumberOptions 后段）：id 为空选首个；liveTitle 为空回填
useEffect(() => {
if (!options.length || optionsWriteBackDoneRef.current) return;
optionsWriteBackDoneRef.current = true;
const d = dataRef.current?? {};
const patch: Record<string, any> = {};
if (!d.videoNumberId) patch.videoNumberId = String(options[0].finderId);
if (!d.liveTitle) {
const currentId = patch.videoNumberId?? d.videoNumberId;
const current = options.find((o) => String(o.finderId) === String(currentId));
patch.liveTitle = current?.liveTitle || DEFAULT_LIVE_TITLE;
}
if (Object.keys(patch).length) onChangeRef.current(patch);
}, [options]);

// 切换到 live 且 liveTitle 为空时回填（同原版 watch contentType）
useEffect(() => {
if (contentType !== "live") return;
const d = dataRef.current?? {};
if (d.liveTitle) return;
const current = options.find((o) => String(o.finderId) === String(d.videoNumberId?? ""));
onChangeRef.current({ liveTitle: current?.liveTitle || DEFAULT_LIVE_TITLE});
}, [contentType, options]);

// liveTitle 超长截断（同原版 watch liveTitle；输入框 onChange 已截断，此处覆盖程序化写入）
useEffect(() => {
const title = data?.liveTitle;
if (typeof title === "string" && title.length > 14) onChangeRef.current({ liveTitle: title.slice(0, 14)});
}, [data?.liveTitle]);

// productImages 规整写回（同原版 watch productImages deep+immediate：上限 3 + 补 picKey，变化才写）
useEffect(() => {
const images = data?.productImages;
if (!Array.isArray(images)) return;
const normalized = images.slice(0, 3).map((item: any, index: number) => ({
...item,
picKey: String(item?.picKey || item?.picId || item?.picUrl || `idx-${index}`)
}));
if (JSON.stringify(normalized) !== JSON.stringify(images)) onChangeRef.current({ productImages: normalized});
}, [data?.productImages]);

return (
<div className="h5-editor-settings-body">
<div style={{ color: "#999", fontSize: 12, lineHeight: "20px", padding: "0 0 12px"}}>{TIP}</div>
<div style={{ display: "flex", margin: "10px 0 16px", borderBottom: "1px solid #ebeef5"}}>
{(
[
{ key: "content", label: "内容设置"},
{ key: "style", label: "样式设置"}
] as const
).map((t) => (
<div
key={t.key}
onClick={() => setTab(t.key)}
style={{
position: "relative",
flex: 1,
padding: "0 4px 12px",
color: tab === t.key? "#155bd4": "#999",
fontWeight: tab === t.key? 600: 400,
fontSize: 14,
cursor: "pointer",
textAlign: "center"
}}
>
{t.label}
{tab === t.key? <span style={{ position: "absolute", left: 0, right: 0, bottom: -1, height: 3, background: "#155bd4"}} />: null}
</div>
))}
</div>
{tab === "content"? (
<div>
<SettingGroup>
<RadioRowField
data={data}
onChange={onChange}
field="contentType"
label="内容类型"
options={[
{ value: "video", label: "视频"},
{ value: "live", label: "直播"}
]}
/>
<SettingRow label="视频号ID" block>
<Select
value={data?.videoNumberId || undefined}
placeholder="请选择需要跳转的视频号ID"
style={{ width: "100%"}}
onChange={(v) => {
const patch: Record<string, any> = { videoNumberId: String(v?? "")};
// 改选视频号时 liveTitle 为空回填所选视频号标题（同原版 handleVideoNumberChange）
if (!dataRef.current?.liveTitle) {
const opt = options.find((o) => String(o.finderId) === String(v?? ""));
patch.liveTitle = opt?.liveTitle || DEFAULT_LIVE_TITLE;
}
onChange(patch);
}}
optionList={options.map((o) => ({ value: String(o.finderId), label: String(o.finderId)}))}
/>
<div style={{ marginTop: 8, color: "#999", fontSize: 12, lineHeight: "18px"}}>跳转的视频号需要与小程序为同主体或者关联主体</div>
</SettingRow>
</SettingGroup>
{contentType === "video"? (
<SettingGroup>
<SettingRow label="视频ID" block>
<Input
value={data?.videoId?? ""}
placeholder="请输入需要跳转的视频ID"
style={{ width: "100%"}}
onChange={(v: string) => onChange({ videoId: v})}
/>
<div style={{ marginTop: 8, color: "#999", fontSize: 12, lineHeight: "18px"}}>
视频ID可在获取
<a href="https://channels.weixin.qq.com/platform" target="_blank" rel="noreferrer" style={{ marginLeft: 4, color: "#155bd4"}}>
去获取
</a>
</div>
</SettingRow>
</SettingGroup>
): (
<>
<SettingGroup>
<SettingRow label="标题" block>
<Input
value={data?.liveTitle?? ""}
maxLength={14}
showClear={false}
placeholder="请输入直播标题"
suffix={<span style={{ color: "#999", fontSize: 12}}>{`${(data?.liveTitle || "").length}/14`}</span>}
style={{ width: "100%"}}
onChange={(v: string) => onChange({ liveTitle: v.slice(0, 14)})}
/>
</SettingRow>
<RadioRowField
data={data}
onChange={onChange}
field="titleColorType"
label="标题颜色"
options={[
{ value: "default", label: "默认"},
{ value: "theme", label: "主题色"},
{ value: "custom", label: "自定义"}
]}
/>
{data?.titleColorType === "custom"? (
<ColorField data={data} onChange={onChange} field="titleColor" label="显示颜色" resetValue="#2A2A2A" />
): null}
</SettingGroup>
<PicListField data={data} onChange={onChange} field="productImages" label="商品图片" max={3} addText="添加图片" />
<div style={{ color: "#999", fontSize: 12, lineHeight: "18px"}}>最多可添加三个图片，支持拖拽排序</div>
</>
)}
</div>
): (
<FrameSettingsField data={data} onChange={onChange} hideHead />
)}
</div>
);
}
