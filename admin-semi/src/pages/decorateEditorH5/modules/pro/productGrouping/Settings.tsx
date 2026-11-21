/**
* 商品分组 设置（对齐 view/admin mobilePro/productGrouping/Settings.vue）
*
* 页签：商品设置 / 扩展设置（商品颜色设置 + 分组标题设置 + ModuleStyle/ContentStyle
* type="productGrouping"）。
* 手动组 GroupListEdit 以共享 GroupListEditor 承接：主标题/副标题/查看更多链接（SelectLink
* 文本输入写回 titleLink 同一结构）/自定义图片（bannerPic PicSelect）可编辑；组内商品
* 走共享 ProductInfoSelect（快照式选品），可增删/更换/拖拽排序组内 productList；
* 新建组默认值与删组下限（可删至 0 组）均按原版 GroupListEdit 口径。
* 商品分组来源走共享 ProductGroupSelect（选分组弹窗 + 换组/拖拽排序/删除/添加，
* 数据源 /product/productGroup/list）：组内 title/subTitle/menuTitle/productNumType/
* productNum/bannerPic 等字段在卡内以 { data, onChange } 子协议编辑，契约不变。
* 列表样式/字号/粗细原版为图标 radio，现走共享视觉选择器
* 。
* bottomMoreBtn 系列原版 Settings.vue 无编辑入口（仅 defaults + Preview 消费），故不提供。
*/
import {
SettingGroup,
SettingRow,
RadioRowField,
RadioField,
BuyBtnStyleField,
RangeField,
ColorField,
TextField,
PicSelectField,
PicLinkField,
DecDivider
} from "../../../shared/settingWidgets";
import { TemplateIconRadioField, TemplateIconRadioRowField} from "../../../shared/templateIcon";
import { SettingsTabs, ExtensionStyleSettings, PicFillTypeField} from "../../../shared/styleEdits";
import GroupListEditor from "../../../shared/GroupListEditor";
import ProductInfoSelect from "../../../shared/ProductInfoSelect";
import ProductGroupSelect from "../../../shared/ProductGroupSelect";
import { cloneDeep, defaultBuyButtonStyle} from "../../../contract/sharedDefaults";
import { useI18n} from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
data: Record<string, any>;
onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
{ value: 1, label: "显示"},
{ value: 0, label: "不显示"}
];
const WEIGHT_ICON_OPTIONS = [
{ value: 400, label: "常规体", aSize: 14},
{ value: 700, label: "加粗体", aSize: 14, aBold: true}
];

function LinkInput({ value, onChangeValue}: { value: any; onChangeValue: (v: any) => void}) {
return <PicLinkField value={value?? null} onChange={(v) => onChangeValue(v)} />;
}

function ManualGroupEditor({ data, onChange}: Props) {
const { t} = useI18n();
return (
<GroupListEditor
data={data}
onChange={onChange}
field="productGroups"
addText="添加一组"
createItem={() => ({
title: "",
subTitle: "",
productList: [],
bannerPic: {},
bannerLink: { link: { link: "", title: ""}, linkTitle: ""},
titleLink: { link: { link: "", title: ""}, linkTitle: ""}
})}
renderItem={(group, _index, itemProps) => {
const sub: Props = itemProps;
const linkValue = group.titleLink?.link && typeof group.titleLink.link === "object"? group.titleLink.link: typeof group.titleLink?.link === "string"? { link: group.titleLink.link, title: "" }: null;
return (
<>
<TextField {...sub} field="title" label="主标题" placeholder="请输入标题内容" />
<TextField {...sub} field="subTitle" label="副标题" placeholder="请输入标题内容" />
<SettingRow label="查看更多链接">
<LinkInput
value={linkValue}
onChangeValue={(v) =>
itemProps.onChange({
titleLink: {...(group.titleLink || {}), link: v}
})
}
/>
</SettingRow>
<PicSelectField {...sub} field="bannerPic" label="自定义图片" />
<SettingRow label="选择商品" block>
<ProductInfoSelect
value={Array.isArray(group.productList)? group.productList: []}
onChange={(next) => itemProps.onChange({ productList: next})}
isMultiple
/>
<div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6, marginTop: 8}}>{t("提示：您可以通过拖拽进行商品排序")}</div>
</SettingRow>
</>
);
}}
/>
);
}

function GroupSourceEditor({ data, onChange}: Props) {
const list: Array<Record<string, any>> = Array.isArray(data?.groupList)? data.groupList: [];
return (
<div style={{ padding: "0 16px 10px"}}>
<ProductGroupSelect
value={list}
onChange={(next) => onChange({ groupList: next})}
isMultiple
type="productGroup"
showCustomImage
/>
</div>
);
}

function BuyBtnFields({ data, onChange}: Props) {
const btn = data?.buyBtnStyle && typeof data.buyBtnStyle === "object"? data.buyBtnStyle: cloneDeep(defaultBuyButtonStyle);
const sub: Props = { data: btn, onChange: (patch: Patch) => onChange({ buyBtnStyle: {...btn,...patch}})};
return (
<>
<DecDivider />
<RadioRowField
{...sub}
field="showBtn"
label="购买按钮"
options={[
{ value: 0, label: "不显示"},
{ value: 1, label: "显示"}
]}
/>
{Number(btn.showBtn) === 1? (
<>
<SettingRow label="购买按钮图标样式" block>
<BuyBtnStyleField {...sub} field="btnType" visualSet="h5BtnType" customIcon={btn.iconImage} />
</SettingRow>
{Number(btn.btnType) === 5? <PicSelectField {...sub} field="iconImage" label="自定义图标图片" />: null}
<RangeField {...sub} field="iconSize" label="图标尺寸" min={12} max={40} />
<TextField {...sub} field="text" label="文字内容" placeholder="请输入按钮文字" />
<RadioRowField
{...sub}
field="wrap"
label="按钮换行"
options={[
{ value: 0, label: "不换行"},
{ value: 1, label: "换行"}
]}
/>
{Number(btn.wrap) === 1? (
<RadioRowField
{...sub}
field="wrapAlign"
label="换行后对齐"
options={[
{ value: 1, label: "居左"},
{ value: 2, label: "居中"},
{ value: 3, label: "居右"}
]}
/>
): null}
<RangeField {...sub} field="textSize" label="文字字号" min={10} max={24} />
<ColorField {...sub} field="textColor" label="文字颜色" resetValue="#ffffff" />
<ColorField {...sub} field="iconColor" label="图标颜色" resetValue="#ffffff" />
<RadioRowField
{...sub}
field="gradientType"
label="背景渐变方向"
options={[
{ value: "upDown", label: "上下"},
{ value: "leftRight", label: "左右"},
{ value: "diagonal", label: "斜向"},
{ value: "purity", label: "纯色"}
]}
/>
<ColorField {...sub} field="gradientColorA" label="背景渐变颜色A" resetValue="#333333" />
<ColorField {...sub} field="gradientColorB" label="背景渐变颜色B" resetValue="#333333" />
<RangeField {...sub} field="radius" label="按钮圆角" max={20} />
<RangeField {...sub} field="paddingTopBottom" label="按钮高度" max={10} />
<RangeField {...sub} field="paddingLeftRight" label="按钮宽度" max={10} />
</>
): null}
</>
);
}

export default function ProductGroupingSettings({ data, onChange }: Props) {
    const pc = data?.productColor ?? {};
    const pcProps: Props = { data: pc, onChange: (patch: Patch) => onChange({ productColor: { ...pc, ...patch } }) };
    const gt = data?.groupTitle ?? {};
    const gtProps: Props = { data: gt, onChange: (patch: Patch) => onChange({ groupTitle: { ...gt, ...patch } }) };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["商品设置", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="productSource"
                            label="商品来源"
                            options={[
                                { value: "products", label: "手动选择" },
                                { value: "productGroup", label: "商品分组" }
                            ]}
                        />
                    </SettingGroup>
                    {data?.productSource === "productGroup" ? (
                        <GroupSourceEditor data={data} onChange={onChange} />
                    ) : (
                        <ManualGroupEditor data={data} onChange={onChange} />
                    )}
                    <SettingGroup title="商品参数设置">
                        <SettingRow label="商品卡片样式" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="productCardStyle"
                                options={[
                                    { value: 1, label: "无边白底" },
                                    { value: 2, label: "卡片投影" },
                                    { value: 3, label: "描边白底" },
                                    { value: 4, label: "无边透明" }
                                ]}
                            />
                        </SettingRow>
                        <TemplateIconRadioField
                            data={data}
                            onChange={onChange}
                            field="listStyle"
                            label="列表样式"
                            options={[
                                { value: 1, label: "大图模式", icon: "up2end" },
                                { value: 2, label: "一行两个", icon: "small" },
                                { value: 3, label: "一行三个", icon: "three" },
                                { value: 4, label: "一行四个", icon: "cuberow1" },
                                { value: 5, label: "一大两小", icon: "hybrid" }
                            ]}
                        />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picScale"
                            label="图片比例"
                            options={[
                                { value: "3:2", label: "3:2" },
                                { value: "1:1", label: "1:1" },
                                { value: "3:4", label: "3:4" },
                                { value: "16:9", label: "16:9" }
                            ]}
                        />
                        <PicFillTypeField data={data} onChange={onChange} field="picFillType" label="图片填充" />
                        <RangeField data={data} onChange={onChange} field="cardMargin" label="商品横竖间距" max={40} />
                        <RangeField data={data} onChange={onChange} field="cardPadding" label="商品内部边距" max={10} />
                        <RangeField data={data} onChange={onChange} field="cardRadius" label="商品圆角半径" max={20} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picBottomRadius"
                            label="图片底部圆角"
                            options={[
                                { value: 1, label: "圆角" },
                                { value: 0, label: "直角" }
                            ]}
                        />
                    </SettingGroup>
                    <DecDivider />
                    <RadioRowField data={data} onChange={onChange} field="showProductName" label="商品标题" options={SHOW_OPTIONS} />
                    {Number(data?.showProductName) === 1 ? (
                        <>
                            <RadioRowField
                                data={data}
                                onChange={onChange}
                                field="productNameRow"
                                label="标题行数"
                                options={[
                                    { value: 1, label: "一行" },
                                    { value: 2, label: "两行" }
                                ]}
                            />
                            <TemplateIconRadioRowField data={data} onChange={onChange} field="productNameWeight" label="标题粗细" options={WEIGHT_ICON_OPTIONS} />
                            <RangeField data={data} onChange={onChange} field="productNameSize" label="标题大小" min={12} max={16} />
                        </>
                    ) : null}
                    <RadioRowField data={data} onChange={onChange} field="showProductPrice" label="商品价格" options={SHOW_OPTIONS} />
                    {Number(data?.showProductPrice) === 1 ? (
                        <>
                            <TemplateIconRadioRowField data={data} onChange={onChange} field="productPriceWeight" label="价格文字粗细" options={WEIGHT_ICON_OPTIONS} />
                            <TemplateIconRadioRowField
                                data={data}
                                onChange={onChange}
                                field="productPriceSize"
                                label="价格文字大小"
                                options={[
                                    { value: 12, label: "小(12号)", aSize: 12, aBold: true },
                                    { value: 14, label: "小(14号)", aSize: 14, aBold: true },
                                    { value: 16, label: "中(16号)", aSize: 16, aBold: true },
                                    { value: 18, label: "大(18号)", aSize: 18, aBold: true }
                                ]}
                            />
                            <PicSelectField data={data} onChange={onChange} field="productPricePic" label="价格标签图(宽不限,高20)" />
                            <RangeField data={data} onChange={onChange} field="productPriceMarginTop" label="价格上距" min={5} max={30} />
                        </>
                    ) : null}
                    <RadioRowField data={data} onChange={onChange} field="crossedOutPrice" label="商品划线价" options={SHOW_OPTIONS} />
                    <BuyBtnFields data={data} onChange={onChange} />
                </>
                <>
                    <SettingGroup title="商品颜色设置">
                        <RangeField {...pcProps} field="infoTopPadding" label="信息区顶部内距" min={4} max={30} />
                        <RangeField {...pcProps} field="infoBottomPadding" label="信息区底部内距" min={4} max={30} />
                        <RangeField {...pcProps} field="infoLeftRightPadding" label="信息区左右内距" min={0} max={20} />
                        <ColorField {...pcProps} field="backgroundColor" label="商品背景颜色" resetValue="" />
                    </SettingGroup>
                    <DecDivider />
                    <ColorField {...pcProps} field="titleColor" label="商品标题颜色" resetValue="" />
                    <ColorField {...pcProps} field="priceColor" label="商品价格颜色" resetValue="" />
                    <ColorField {...pcProps} field="crossedOutPriceColor" label="商品划线价颜色" resetValue="" />
                    <DecDivider />
                    <ColorField {...pcProps} field="moreBtnBgColor" label="查看更多背景颜色" resetValue="" />
                    <ColorField {...pcProps} field="moreBtnBorderColor" label="查看更多边框颜色" resetValue="" />
                    <ColorField {...pcProps} field="moreBtnTextColor" label="查看更多文字颜色" resetValue="" />
                    <SettingGroup title="分组标题设置">
                        <RangeField {...gtProps} field="marginLeftRight" label="分组标题间距" min={0} max={20} />
                        <RadioRowField {...gtProps} field="showTitle" label="分组大标题" options={SHOW_OPTIONS} />
                        {Number(gt.showTitle) === 1 ? (
                            <>
                                <TemplateIconRadioRowField
                                    {...gtProps}
                                    field="bigTitleFontSize"
                                    label="大标题大小"
                                    options={[
                                        { value: 14, label: "小(14号)", aSize: 14, aBold: true },
                                        { value: 16, label: "中(16号)", aSize: 16, aBold: true },
                                        { value: 18, label: "大(18号)", aSize: 18, aBold: true }
                                    ]}
                                />
                                <ColorField {...gtProps} field="titleColor" label="大标题默认颜色" resetValue="" />
                                <ColorField {...gtProps} field="activeTitleColor" label="大标题选中颜色" resetValue="" />
                            </>
                        ) : null}
                        <RadioRowField {...gtProps} field="showSubTitle" label="分组副标题" options={SHOW_OPTIONS} />
                        {Number(gt.showSubTitle) === 1 ? (
                            <>
                                <TemplateIconRadioRowField
                                    {...gtProps}
                                    field="titleFontSize"
                                    label="副标题大小"
                                    options={[
                                        { value: 12, label: "小(12号)", aSize: 12, aBold: true },
                                        { value: 14, label: "中(14号)", aSize: 14, aBold: true },
                                        { value: 16, label: "大(16号)", aSize: 16, aBold: true }
                                    ]}
                                />
                                <ColorField {...gtProps} field="subTitleColor" label="副标题默认颜色" resetValue="" />
                                <ColorField {...gtProps} field="activeSubTitleColor" label="副标题选中颜色" resetValue="" />
                                <RadioRowField
                                    {...gtProps}
                                    field="subTitleGradientType"
                                    label="副标题渐变方向"
                                    options={[
                                        { value: "upDown", label: "上下" },
                                        { value: "leftRight", label: "左右" },
                                        { value: "diagonal", label: "斜向" }
                                    ]}
                                />
                                <ColorField {...gtProps} field="gradientColorA" label="副标题渐变颜色A" resetValue="#72b76c" />
                                <ColorField {...gtProps} field="gradientColorB" label="副标题渐变颜色B" resetValue="#72b76c" />
                                <RangeField {...gtProps} field="subTitleRadius" label="副标题圆角半径" max={15} />
                            </>
                        ) : null}
                        <ColorField {...gtProps} field="splitLineColor" label="标题分割线颜色" resetValue="" />
                    </SettingGroup>
                    <ExtensionStyleSettings data={data} onChange={onChange} type="productGrouping" />
                </>
            </SettingsTabs>
        </div>
    );
}
