/**
 * 首焦轮播 设置（对齐 view/admin mobilePro/banner/Settings.vue + 共享
 * BannerContent/BannerSearch/BannerNav/BannerStyle 四个子编辑组件）
 *
 * 页签：基础设置 / 样式设置 / 扩展设置。
 * 基础：商城logo（PicSelect + 图片高度 max500）+ BannerContent（PicList/指示器样式/指示器颜色/
 * 轮播圆角 max50/弧度背景高度 max500/弧度背景圆角 max100/弧度背景方向）+ BannerSearch
 * （显示开关/默认搜索词/菜单图标 1-7+自定义+不显示/菜单链接/签到图标 1-4+自定义+不显示/签到链接）
 * + BannerNav（显示开关/文字列表）；样式：BannerStyle（type 非 bannerA2 全集）；
 * 扩展：ModuleStyleEdit（type="banner"，含 bannerMarginTop）。
 * 注：SelectLink（菜单/签到跳转链接）以文本输入承接，写回原字段（字符串型，与原版默认值同形）；
 * BannerContent 的 PicList 原版带 showColor（逐图渐变色），我方 PicListField 无逐项颜色，渐变色保持原数据。
 */
import { Input } from "@douyinfe/semi-ui";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RangeField,
    ColorField,
    TextField,
    PicListField,
    PicSelectField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField } from "../../../shared/styleEdits";

type NestedProps = { data: Record<string, any>; onChange: (patch: Record<string, any>) => void };
function nested(data: Record<string, any>, onChange: (patch: Record<string, any>) => void, key: string): NestedProps {
    const sub = (data?.[key] ?? {}) as Record<string, any>;
    return { data: sub, onChange: (patch: Record<string, any>) => onChange({ [key]: { ...sub, ...patch } }) };
}

function LinkTextField({ data, onChange, field, label }: NestedProps & { field: string; label: string }) {
    return (
        <SettingRow label={label}>
            <Input
                value={typeof data?.[field] === "string" ? data[field] : data?.[field]?.link ?? ""}
                placeholder="跳转链接，如 /pages/index/index"
                style={{ width: 240, maxWidth: "100%" }}
                onChange={(v: string) => onChange({ [field]: v })}
            />
        </SettingRow>
    );
}

export default function BannerSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
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
                        <PicListField {...bc} field="picList" label="" max={20} addText="添加一组" />
                        <RadioRowField
                            {...bc}
                            field="picPageType"
                            label="指示器样式"
                            options={[
                                { value: 1, label: "样式一" },
                                { value: 2, label: "样式二" }
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
                                { value: "upDown", label: "上下" },
                                { value: "leftRight", label: "左右" },
                                { value: "diagonal", label: "斜向" }
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
                                { value: 1, label: "显示" },
                                { value: 0, label: "不显示" }
                            ]}
                        />
                        {showSearch ? (
                            <>
                                <TextField {...sc} field="searchText" label="默认搜索词" placeholder="请输入搜索框文字" />
                                <RadioRowField
                                    {...sc}
                                    field="menuIcon"
                                    label="菜单图标"
                                    options={[
                                        ...[1, 2, 3, 4, 5, 6, 7].map((n) => ({ value: n, label: `图标${n}` })),
                                        { value: "custom", label: "自定义" },
                                        { value: 0, label: "不显示" }
                                    ]}
                                />
                                {sc.data.menuIcon === "custom" ? <PicSelectField {...sc} field="menuPic" label="自定义菜单图标" /> : null}
                                {Number(sc.data.menuIcon) !== 0 && sc.data.menuIcon !== undefined ? (
                                    <LinkTextField {...sc} field="menuLink" label="菜单跳转链接" />
                                ) : null}
                                <RadioRowField
                                    {...sc}
                                    field="signIcon"
                                    label="签到图标"
                                    options={[
                                        ...[1, 2, 3, 4].map((n) => ({ value: n, label: `图标${n}` })),
                                        { value: "custom", label: "自定义" },
                                        { value: 0, label: "不显示" }
                                    ]}
                                />
                                {sc.data.signIcon === "custom" ? <PicSelectField {...sc} field="signPic" label="自定义签到图标" /> : null}
                                {Number(sc.data.signIcon) !== 0 && sc.data.signIcon !== undefined ? (
                                    <LinkTextField {...sc} field="signLink" label="签到跳转链接" />
                                ) : null}
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
                        {Number(nc.data.showNav) === 1 ? <PicListField {...nc} field="navList" label="" max={20} showText addText="添加文字和链接" /> : null}
                    </SettingGroup>
                </>
                <>
                    <RadioRowField
                        {...bs}
                        field="effectType"
                        label="搜索框/导航效果"
                        options={[
                            { value: "default", label: "正常模式" },
                            { value: "ceiling", label: "滚动至顶部固定" }
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
                    <RadioRowField
                        {...bs}
                        field="textAlignment"
                        label="导航文字对齐"
                        options={[
                            { value: "left", label: "左对齐" },
                            { value: "center", label: "居中对齐" },
                            { value: "right", label: "右对齐" }
                        ]}
                    />
                    <RadioRowField
                        {...bs}
                        field="navFontBold"
                        label="导航文字粗细"
                        options={[
                            { value: 400, label: "常规体" },
                            { value: 700, label: "加粗体" }
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
