/**
* 首焦轮播 设置（对齐 view/admin mobilePro/banner/Settings.vue + 共享
* BannerContent/BannerSearch/BannerNav/BannerStyle 四个子编辑组件）
*
* 页签：基础设置 / 样式设置 / 扩展设置。
* 基础：商城logo（PicSelect + 图片高度 max500）+ BannerContent（PicList/指示器样式/指示器颜色/
* 轮播圆角 max50/弧度背景高度 max500/弧度背景圆角 max100/弧度背景方向）+ BannerSearch
* （显示开关/默认搜索词/菜单图标 1-7+自定义+不显示/菜单链接/签到图标 1-4+自定义+不显示/签到链接
* ／多语言图标显示开关 showLang——我方新增：原版无此设置、多语言图标由前端无条件渲染）
* + BannerNav（显示开关/文字列表，PicList showImage=false 同原版：只有文字+链接、无图片字段）；
* 样式：BannerStyle（type 非 bannerA2 全集）；
* 扩展：ModuleStyleEdit（type="banner"，含 bannerMarginTop）。
* 注：SelectLink（菜单/签到跳转链接）走共享 PicLinkField 选择，写回原字段（字符串型，与原版默认值同形）；
* BannerContent 的 PicList 原版带 showColor（逐图渐变色），我方 PicListField 无逐项颜色，渐变色保持原数据。
* 指示器样式（共享 BannerContent）、搜索框/导航效果/导航文字对齐/导航文字粗细
* （BannerStyle）原版均为图标 radio，2026-11-20 起走共享视觉选择器
* 。
*/
import {
SettingGroup,
SettingRow,
RadioRowField,
IconStyleRadioField,
RangeField,
ColorField,
TextField,
PicListField,
PicListEditorField,
PicSelectField,
PicLinkField,
DecDivider
} from "../../../shared/settingWidgets";
import { TemplateIconRadioRowField} from "../../../shared/templateIcon";
import { SettingsTabs, ModuleStyleSettingsField} from "../../../shared/styleEdits";

// 菜单/签到默认图标字形：IconPark 映射（./iconStyles.ts，全站唯一一份，与 Preview 共用；
// 不再使用原版 defaultIconImage 的 tigshop 默认图——全站图标换 IconPark 的既定改口径）
import { MENU_ICON_PARK, SIGN_ICON_PARK} from "./iconStyles";

type NestedProps = { data: Record<string, any>; onChange: (patch: Record<string, any>) => void};
function nested(data: Record<string, any>, onChange: (patch: Record<string, any>) => void, key: string): NestedProps {
const sub = (data?.[key]?? {}) as Record<string, any>;
return { data: sub, onChange: (patch: Record<string, any>) => onChange({ [key]: {...sub,...patch}})};
}

function LinkTextField({ data, onChange, field, label}: NestedProps & { field: string; label: string}) {
// 存量字段为字符串 path（渲染端亦兼容 {link} 对象）：经 PicLinkField 选择后仍只写回 path 字符串，数据形状不变
const url = typeof data?.[field] === "string"? data[field]: data?.[field]?.link?? "";
return (
<SettingRow label={label}>
<PicLinkField value={{ link: url, title: ""}} onChange={(v) => onChange({ [field]: v.link})} />
</SettingRow>
);
}

export default function BannerSettings({ data, onChange}: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void}) {
const bc = nested(data, onChange, "bannerContent");
const sc = nested(data, onChange, "searchContent");
const nc = nested(data, onChange, "navContent");
const bs = nested(data, onChange, "bannerStyle");
const showSearch = Number(sc.data.showSearch) === 1;
const showBtn = Number(bs.data.showSearchBtn) === 1;

return (
<div className="h5-editor-settings-body">
<SettingsTabs tabs={["基础设置", "样式设置", "扩展设置"]}>
<>
<SettingGroup title="商城logo设置" variant="spread">
<PicSelectField data={data} onChange={onChange} field="logoPic" label="图片设置" />
<RangeField data={data} onChange={onChange} field="logoHeight" label="图片高度" max={500} />
</SettingGroup>
<SettingGroup title="轮播内容设置" variant="spread">
<PicListEditorField {...bc} field="picList" label="" max={20} addText="添加一组" />
<TemplateIconRadioRowField
{...bc}
field="picPageType"
label="指示器样式"
options={[
{ value: 1, label: "样式一", icon: "indicator-1"},
{ value: 2, label: "样式二", icon: "indicator-2"}
]}
/>
<ColorField {...bc} field="swiperPageColor" label="指示器颜色" resetValue="" />
<RangeField {...bc} field="picRadius" label="轮播图片圆角" max={50} />
<RangeField {...bc} field="height" label="弧度背景高度" max={500} />
<RangeField {...bc} field="backgroundRadius" label="弧度背景圆角" max={100} />
<RadioRowField
{...bc}
field="gradientType"
label="弧度背景方向"
options={[
{ value: "upDown", label: "上下"},
{ value: "leftRight", label: "左右"},
{ value: "diagonal", label: "斜向"}
]}
/>
</SettingGroup>
<DecDivider />
<SettingGroup title="搜索框设置" variant="spread">
<RadioRowField
{...sc}
field="showSearch"
label="搜索框设置"
options={[
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"}
]}
/>
{showSearch? (
<>
<TextField {...sc} field="searchText" label="默认搜索词" placeholder="请输入搜索框文字" />
<IconStyleRadioField
{...sc}
field="menuIcon"
label="菜单图标"
options={[
...MENU_ICON_PARK.map((name, i) => ({ value: i + 1, label: `图标${i + 1}`, iconpark: name})),
{ value: "custom", label: "自定义"},
{ value: 0, label: "不显示"}
]}
/>
{sc.data.menuIcon === "custom"? <PicSelectField {...sc} field="menuPic" label="自定义菜单图标" />: null}
{Number(sc.data.menuIcon)!== 0 && sc.data.menuIcon!== undefined? (
<LinkTextField {...sc} field="menuLink" label="菜单跳转链接" />
): null}
                                <IconStyleRadioField
                                    {...sc}
                                    field="signIcon"
                                    label="签到图标"
                                    options={[
                                        ...SIGN_ICON_PARK.map((name, i) => ({ value: i + 1, label: `图标${i + 1}`, iconpark: name })),
                                        { value: "custom", label: "自定义" },
                                        { value: 0, label: "不显示" }
                                    ]}
                                />
                                {sc.data.signIcon === "custom" ? <PicSelectField {...sc} field="signPic" label="自定义签到图标" /> : null}
                                {Number(sc.data.signIcon) !== 0 && sc.data.signIcon !== undefined ? (
                                    <LinkTextField {...sc} field="signLink" label="签到跳转链接" />
                                ) : null}
                                {/* 多语言图标显示开关（我方新增）：存量数据无 showLang 字段时按默认 1 呈现选中态，
                                    前端同口径（showLang ?? 1 显示）；排布在菜单/签到之后，与搜索行左→右顺序一致 */}
                                <RadioRowField
                                    data={{ ...sc.data, showLang: sc.data.showLang ?? 1 }}
                                    onChange={sc.onChange}
                                    field="showLang"
                                    label="多语言图标"
                                    options={[
                                        { value: 1, label: "显示" },
                                        { value: 0, label: "不显示" }
                                    ]}
                                />
                            </>
                        ) : null}
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="导航文字设置" variant="spread">
                        <RadioRowField
                            {...nc}
                            field="showNav"
                            label="导航文字设置"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        {Number(nc.data.showNav) === 1 ? (
                            <PicListField {...nc} field="navList" label="" max={20} showText showImage={false} addText="添加文字和链接" />
                        ) : null}
                    </SettingGroup>
                </>
                <>
                    <TemplateIconRadioRowField
                        {...bs}
                        field="effectType"
                        label="搜索框/导航效果"
                        options={[
                            { value: "default", label: "正常模式", icon: "effect-default" },
                            { value: "ceiling", label: "滚动至顶部固定", icon: "effect-ceiling" }
                        ]}
                    />
                    <DecDivider />
                    <ColorField {...bs} field="searchBackgroundColor" label="背景颜色" resetValue="" />
                    <ColorField {...bs} field="searchFontColor" label="文本颜色" resetValue="" />
                    <RadioRowField
                        {...bs}
                        field="showSearchBtn"
                        label="是否显示按钮"
                        options={[
                            { value: 1, label: "显示" },
                            { value: 0, label: "不显示" }
                        ]}
                    />
                    {showBtn ? <TextField {...bs} field="searchText" label="按钮文本" placeholder="请输入按钮文本" /> : null}
                    <RangeField {...bs} field="searchHeight" label="搜索框高度" max={40} />
                    <RangeField {...bs} field="searchRadius" label="搜索框圆角" max={20} />
                    <RangeField {...bs} field="searchIconSize" label="图标尺寸" max={35} />
                    <DecDivider />
                    <TextField {...bs} field="navPrefix" label="导航文字前缀" placeholder="请输入导航文字前缀" />
                    <ColorField {...bs} field="navFontColor" label="导航文字颜色" resetValue="" />
                    <ColorField {...bs} field="navBackgroundColor" label="背景颜色" resetValue="" />
                    <ColorField {...bs} field="navPrefixColor" label="文字前缀颜色" resetValue="" />
                    <TemplateIconRadioRowField
                        {...bs}
                        field="textAlignment"
                        label="导航文字对齐"
                        options={[
                            { value: "left", label: "左对齐", icon: "align-left" },
                            { value: "center", label: "居中对齐", icon: "align-center" },
                            { value: "right", label: "右对齐", icon: "align-right" }
                        ]}
                    />
                    <TemplateIconRadioRowField
                        {...bs}
                        field="navFontBold"
                        label="导航文字粗细"
                        options={[
                            { value: 400, label: "常规体", aSize: 14 },
                            { value: 700, label: "加粗体", aSize: 14, aBold: true }
                        ]}
                    />
                    <RangeField {...bs} field="navLabelMargin" label="导航文字间距" max={20} />
                    <RangeField {...bs} field="navFontSize" label="导航文字大小" max={20} />
                </>
                <ModuleStyleSettingsField data={data} onChange={onChange} type="banner" />
            </SettingsTabs>
        </div>
    );
}
