/**
 * 分组滑动 设置（对齐 view/admin mobilePro/groupSliding/Settings.vue）
 *
 * 页签：商品设置 / 扩展设置。
 * 商品设置：分组卡（大标题/副标题/标题链接/商品标签多行/背景颜色）由共享
 * GroupListEditor（分组列表编辑器）承接；组内商品走共享 ProductInfoSelect
 * 快照式选品（可增删/更换/拖拽排序，2026-11-21 接入，此前为未接入只读提示）；
 * 标题链接走共享 PicLinkField（原版 SelectLink 口径，此前为裸文本框且写回
 * 形状会把扁平 titleLink 压成混合嵌套形）+ 参数设置
 * （分组宽度 60-100/分组圆角 max40/图片占比 20-50/图片圆角 max20/右侧圆角/排名/标签/
 * 商品标题组/商品价格组/划线价）+ BuyBtnEdit（buyBtnStyle 对象编辑，字段对齐 buyBtn/Edit.vue）；
 * 扩展设置：颜色设置（moduleColor 嵌套）+ ModuleStyleEdit（type="categoryRec"）。
 * 注：原版 defaults 的价格粗细/字号键名为 productPrice_weight / productPrice_size（下划线），
 * 而原版 Settings.vue 绑定的是 productPriceWeight / productPriceSize（另一组键名，原版内部不一致）；
 * 此处绑定契约键 productPrice_weight / productPrice_size，以源码 defaults 为准（见交付摘要）。
 */
import { TextArea } from "@douyinfe/semi-ui";
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    BuyBtnStyleRowField,
    RangeField,
    ColorField,
    TextField,
    PicSelectField,
    PicLinkField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField } from "../../../shared/styleEdits";
import GroupListEditor from "../../../shared/GroupListEditor";
import ProductInfoSelect from "../../../shared/ProductInfoSelect";
import { defaultBuyButtonStyle } from "../../../contract/sharedDefaults";
import { resolveObjBuyButton } from "../../../../../components/buyStyleObj";
import { usePcGlobalBuyStyle } from "../../../../decoratePc/previews/shared/PcBuyButton";
import BuyBtnFollowRow from "../../../shared/BuyBtnFollowRow";

type NestedProps = { data: Record<string, any>; onChange: (patch: Record<string, any>) => void };
function nested(data: Record<string, any>, onChange: (patch: Record<string, any>) => void, key: string): NestedProps {
    const sub = (data?.[key] ?? {}) as Record<string, any>;
    return { data: sub, onChange: (patch: Record<string, any>) => onChange({ [key]: { ...sub, ...patch } }) };
}

/** 新组模板：titleLink 与模块 defaults 同为扁平 { link, title } 契约（PicLinkField 空契约同形） */
const EMPTY_GROUP = {
    title: "",
    subTitle: "",
    label: "",
    backgroundColor: "",
    productList: [],
    titleLink: { link: "", title: "" }
};

function BuyBtnSettingsField({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const bb = nested(data, onChange, "buyBtnStyle");
    const sub = bb.data;
    // 跟随全局：开启时下方字段显示全局当前值（effSub）并置灰只读；写回仍走
    // bb.onChange 合并进模块原对象，全局值绝不写进模块数据。
    const global = usePcGlobalBuyStyle();
    const following = Number(sub.followGlobal) === 1;
    const effSub = following ? resolveObjBuyButton(sub, global) : sub;
    const bbEff = { data: effSub, onChange: bb.onChange };
    return (
        <SettingGroup title="购买按钮">
            <RadioRowField
                {...bb}
                field="showBtn"
                label="购买按钮"
                options={[
                    { value: 0, label: "不显示" },
                    { value: 1, label: "显示" }
                ]}
            />
            {Number(sub.showBtn) === 1 ? (
                <>
                    <BuyBtnFollowRow data={sub} onChange={bb.onChange} />
                    <div style={following ? { opacity: 0.55, pointerEvents: "none" } : undefined}>
                    <BuyBtnStyleRowField {...bbEff} field="btnType" label="购买按钮图标样式" visualSet="h5BtnType" customIcon={effSub.iconImage} />
                    {Number(effSub.btnType) === 5 ? <PicSelectField {...bbEff} field="iconImage" label="自定义图标图片" /> : null}
                    <RangeField {...bbEff} field="iconSize" label="图标尺寸" min={12} max={40} />
                    <TextField {...bbEff} field="text" label="文字内容" placeholder="请输入按钮文字内容" />
                    <RadioRowField
                        {...bbEff}
                        field="wrap"
                        label="按钮换行"
                        options={[
                            { value: 0, label: "不换行" },
                            { value: 1, label: "换行" }
                        ]}
                    />
                    {Number(sub.wrap) === 1 ? (
                        <RadioRowField
                            {...bbEff}
                            field="wrapAlign"
                            label="换行后对齐"
                            options={[
                                { value: 1, label: "居左" },
                                { value: 2, label: "居中" },
                                { value: 3, label: "居右" }
                            ]}
                        />
                    ) : null}
                    <RangeField {...bbEff} field="textSize" label="文字字号" min={10} max={24} />
                    <ColorField {...bbEff} field="textColor" label="文字颜色" resetValue={defaultBuyButtonStyle.textColor} />
                    <ColorField {...bbEff} field="iconColor" label="图标颜色" resetValue={defaultBuyButtonStyle.iconColor} />
                    <RadioRowField
                        {...bbEff}
                        field="gradientType"
                        label="背景渐变方向"
                        options={[
                            { value: "upDown", label: "上下" },
                            { value: "leftRight", label: "左右" },
                            { value: "diagonal", label: "斜向" },
                            { value: "purity", label: "纯色" }
                        ]}
                    />
                    <SettingRow label="背景渐变颜色">
                        <div className="dec-color-group">
                            <div className="dec-color-button">
                                <a
                                    className="dec-color-reset"
                                    onClick={() => bb.onChange({ gradientColorA: defaultBuyButtonStyle.gradientColorA, gradientColorB: defaultBuyButtonStyle.gradientColorB })}
                                >
                                    重置
                                </a>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput value={String(effSub.gradientColorA ?? "")} onChange={(v) => bb.onChange({ gradientColorA: v })} />
                                </span>
                                <span style={{ margin: "0 6px", color: "#969799" }}>-</span>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput value={String(effSub.gradientColorB ?? "")} onChange={(v) => bb.onChange({ gradientColorB: v })} />
                                </span>
                            </div>
                        </div>
                    </SettingRow>
                    <RangeField {...bbEff} field="radius" label="按钮圆角" max={20} />
                    <RangeField {...bbEff} field="paddingTopBottom" label="按钮高度" max={10} />
                    {effSub.text !== "" && effSub.text !== undefined ? <RangeField {...bbEff} field="paddingLeftRight" label="按钮宽度" max={10} /> : null}
                    </div>
                </>
            ) : null}
        </SettingGroup>
    );
}

const SHOW_HIDE = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

export default function GroupSlidingSettings({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const mc = nested(data, onChange, "moduleColor");
    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["商品设置", "扩展设置"]}>
                <>
                    <SettingGroup title="内容设置">
                        <GroupListEditor
                            data={data}
                            onChange={onChange}
                            field="groupList"
                            addText="添加一组"
                            emptyItem={EMPTY_GROUP}
                            renderItem={(item, _index, itemProps) => (
                                <>
                                    <TextField {...itemProps} field="title" label="分组大标题" placeholder="请输入标题内容" />
                                    <TextField {...itemProps} field="subTitle" label="分组副标题" placeholder="请输入标题内容" />
                                    <SettingRow label="标题链接">
                                        <PicLinkField value={item.titleLink ?? null} onChange={(v) => itemProps.onChange({ titleLink: v })} />
                                    </SettingRow>
                                    <SettingRow label="选择商品" block>
                                        <ProductInfoSelect
                                            value={Array.isArray(item.productList) ? item.productList : []}
                                            onChange={(next) => itemProps.onChange({ productList: next })}
                                            isMultiple
                                        />
                                        <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6, marginTop: 8 }}>
                                            提示：您可以通过拖拽进行商品排序
                                        </div>
                                    </SettingRow>
                                    <SettingRow label="商品标签" block>
                                        <TextArea
                                            value={item.label ?? ""}
                                            rows={4}
                                            placeholder="请输入商品标签"
                                            style={{ width: "100%" }}
                                            onChange={(v: string) => itemProps.onChange({ label: v })}
                                        />
                                    </SettingRow>
                                    <ColorField {...itemProps} field="backgroundColor" label="背景颜色" resetValue="" />
                                </>
                            )}
                        />
                    </SettingGroup>
                    <SettingGroup title="参数设置">
                        <RangeField data={data} onChange={onChange} field="groupWidth" label="分组展示宽度" min={60} max={100} />
                        <RangeField data={data} onChange={onChange} field="groupRadius" label="分组上下圆角" max={40} />
                        <RangeField data={data} onChange={onChange} field="productPicRatio" label="商品图片占比" min={20} max={50} />
                        <RangeField data={data} onChange={onChange} field="productPicRadius" label="商品图片圆角" max={20} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="productPicRightRadius"
                            label="图片右侧圆角"
                            options={[
                                { value: 1, label: "圆角" },
                                { value: 0, label: "直角" }
                            ]}
                        />
                        <DecDivider />
                        <RadioRowField data={data} onChange={onChange} field="showRanking" label="排名标签" options={SHOW_HIDE} />
                        <RadioRowField data={data} onChange={onChange} field="showProductLabel" label="商品标签" options={SHOW_HIDE} />
                        <RadioRowField data={data} onChange={onChange} field="showProductName" label="商品标题" options={SHOW_HIDE} />
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
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="productNameWeight"
                                    label="标题粗细"
                                    options={[
                                        { value: 400, label: "常规体" },
                                        { value: 700, label: "加粗体" }
                                    ]}
                                />
                                <RangeField data={data} onChange={onChange} field="productNameSize" label="标题大小" min={12} max={16} />
                            </>
                        ) : null}
                        <RadioRowField data={data} onChange={onChange} field="showProductPrice" label="商品价格" options={SHOW_HIDE} />
                        {Number(data?.showProductPrice) === 1 ? (
                            <>
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="productPrice_weight"
                                    label="价格文字粗细"
                                    options={[
                                        { value: 400, label: "常规体" },
                                        { value: 700, label: "加粗体" }
                                    ]}
                                />
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="productPrice_size"
                                    label="价格文字大小"
                                    options={[
                                        { value: 12, label: "小(12号)" },
                                        { value: 14, label: "小(14号)" },
                                        { value: 16, label: "中(16号)" },
                                        { value: 18, label: "大(18号)" }
                                    ]}
                                />
                                <RangeField data={data} onChange={onChange} field="productPriceMarginTop" label="价格上距" min={5} max={30} />
                            </>
                        ) : null}
                        <RadioRowField data={data} onChange={onChange} field="crossedOutPrice" label="商品划线价" options={SHOW_HIDE} />
                    </SettingGroup>
                    <BuyBtnSettingsField data={data} onChange={onChange} />
                </>
                <>
                    <SettingGroup title="颜色设置">
                        <RangeField {...mc} field="titleSize" label="分组大标题大小" min={12} max={24} />
                        <RangeField {...mc} field="subTitleSize" label="分组副标题大小" min={12} max={16} />
                        <ColorField {...mc} field="titleColor" label="大标题颜色" resetValue="" />
                        <ColorField {...mc} field="subTitleColor" label="副标题颜色" resetValue="" />
                        <PicSelectField {...mc} field="titlePic" label="标题图标：宽不限, 高40" />
                        <DecDivider />
                        <ColorField {...mc} field="productBackgroundColor" label="商品背景颜色" resetValue="" />
                        <ColorField {...mc} field="productNameColor" label="商品标题颜色" resetValue="" />
                        <ColorField {...mc} field="labelBackgroundColor" label="标签背景颜色" resetValue="" />
                        <ColorField {...mc} field="labelTextColor" label="标签文字颜色" resetValue="" />
                        <ColorField {...mc} field="productPriceColor" label="商品价格颜色" resetValue="" />
                        <ColorField {...mc} field="crossedOutPriceColor" label="商品划线价颜色" resetValue="" />
                    </SettingGroup>
                    <ModuleStyleSettingsField data={data} onChange={onChange} type="categoryRec" />
                </>
            </SettingsTabs>
        </div>
    );
}
