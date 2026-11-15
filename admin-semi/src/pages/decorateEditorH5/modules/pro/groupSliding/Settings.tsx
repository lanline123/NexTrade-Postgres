/**
 * 分组滑动 设置（对齐 view/admin mobilePro/groupSliding/Settings.vue）
 *
 * 页签：商品设置 / 扩展设置。
 * 商品设置：GroupListEdit 分组卡（大标题/副标题/标题链接/商品标签多行/背景颜色；
 * 组内商品为 ProductInfoSelect 数据选择器，未接入 → 只读提示）+ 参数设置
 * （分组宽度 60-100/分组圆角 max40/图片占比 20-50/图片圆角 max20/右侧圆角/排名/标签/
 * 商品标题组/商品价格组/划线价）+ BuyBtnEdit（buyBtnStyle 对象编辑，字段对齐 buyBtn/Edit.vue）；
 * 扩展设置：颜色设置（moduleColor 嵌套）+ ModuleStyleEdit（type="categoryRec"）。
 * 注：原版 defaults 的价格粗细/字号键名为 productPrice_weight / productPrice_size（下划线），
 * 而原版 Settings.vue 绑定的是 productPriceWeight / productPriceSize（另一组键名，原版内部不一致）；
 * 此处绑定契约键 productPrice_weight / productPrice_size，以源码 defaults 为准（见交付摘要）。
 */
import { Input, TextArea } from "@douyinfe/semi-ui";
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RangeField,
    ColorField,
    TextField,
    PicSelectField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs, ModuleStyleSettingsField, GroupCardList } from "../../../shared/styleEdits";
import { defaultBuyButtonStyle } from "../../../contract/sharedDefaults";

type NestedProps = { data: Record<string, any>; onChange: (patch: Record<string, any>) => void };
function nested(data: Record<string, any>, onChange: (patch: Record<string, any>) => void, key: string): NestedProps {
    const sub = (data?.[key] ?? {}) as Record<string, any>;
    return { data: sub, onChange: (patch: Record<string, any>) => onChange({ [key]: { ...sub, ...patch } }) };
}

const EMPTY_GROUP = {
    title: "",
    subTitle: "",
    label: "",
    backgroundColor: "",
    productList: [],
    titleLink: { link: { link: "", title: "" }, linkTitle: "" }
};

function GroupListEditor({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const list: Array<Record<string, any>> = Array.isArray(data?.groupList) ? data.groupList : [];
    const setList = (next: Array<Record<string, any>>) => onChange({ groupList: next });
    const patchItem = (index: number, patch: Record<string, any>) => setList(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
    const moveItem = (index: number, delta: number) => {
        const target = index + delta;
        if (target < 0 || target >= list.length) return;
        const next = [...list];
        const [it] = next.splice(index, 1);
        next.splice(target, 0, it);
        setList(next);
    };
    return (
        <GroupCardList
            items={list}
            addText="添加一组"
            onAdd={() => setList([...list, JSON.parse(JSON.stringify(EMPTY_GROUP))])}
            onRemove={(index) => setList(list.filter((_, i) => i !== index))}
            onMove={moveItem}
            renderItem={(item, index) => {
                const itemProps = { data: item, onChange: (patch: Record<string, any>) => patchItem(index, patch) };
                const linkUrl = item.titleLink?.link?.link ?? (typeof item.titleLink === "string" ? item.titleLink : "");
                return (
                    <>
                        <TextField {...itemProps} field="title" label="分组大标题" placeholder="请输入标题内容" />
                        <TextField {...itemProps} field="subTitle" label="分组副标题" placeholder="请输入标题内容" />
                        <SettingRow label="标题链接">
                            <Input
                                value={linkUrl}
                                placeholder="跳转链接，如 /pages/index/index"
                                style={{ width: 240, maxWidth: "100%" }}
                                onChange={(v: string) =>
                                    patchItem(index, {
                                        titleLink:
                                            item.titleLink && typeof item.titleLink === "object"
                                                ? { ...item.titleLink, link: { ...(item.titleLink.link || { title: "" }), link: v } }
                                                : { link: { link: v, title: "" }, linkTitle: "" }
                                    })
                                }
                            />
                        </SettingRow>
                        <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6, padding: "4px 16px" }}>
                            商品选择器（ProductInfoSelect 数据选择组件）尚未接入本编辑器，已选商品（{(item.productList || []).length} 个）数据保持不变。
                            <br />
                            提示：您可以通过拖拽进行商品排序
                        </div>
                        <SettingRow label="商品标签" block>
                            <TextArea
                                value={item.label ?? ""}
                                rows={4}
                                placeholder="请输入商品标签"
                                style={{ width: "100%" }}
                                onChange={(v: string) => patchItem(index, { label: v })}
                            />
                        </SettingRow>
                        <ColorField {...itemProps} field="backgroundColor" label="背景颜色" resetValue="" />
                    </>
                );
            }}
        />
    );
}

function BuyBtnSettingsField({ data, onChange }: { data: Record<string, any>; onChange: (patch: Record<string, any>) => void }) {
    const bb = nested(data, onChange, "buyBtnStyle");
    const sub = bb.data;
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
                    <RadioRowField
                        {...bb}
                        field="btnType"
                        label="购买按钮图标样式"
                        options={[
                            { value: 1, label: "样式一" },
                            { value: 2, label: "样式二" },
                            { value: 3, label: "样式三" },
                            { value: 4, label: "样式四" },
                            { value: 5, label: "自定义图片" }
                        ]}
                    />
                    {Number(sub.btnType) === 5 ? <PicSelectField {...bb} field="iconImage" label="自定义图标图片" /> : null}
                    <RangeField {...bb} field="iconSize" label="图标尺寸" min={12} max={40} />
                    <TextField {...bb} field="text" label="文字内容" placeholder="请输入按钮文字内容" />
                    <RadioRowField
                        {...bb}
                        field="wrap"
                        label="按钮换行"
                        options={[
                            { value: 0, label: "不换行" },
                            { value: 1, label: "换行" }
                        ]}
                    />
                    {Number(sub.wrap) === 1 ? (
                        <RadioRowField
                            {...bb}
                            field="wrapAlign"
                            label="换行后对齐"
                            options={[
                                { value: 1, label: "居左" },
                                { value: 2, label: "居中" },
                                { value: 3, label: "居右" }
                            ]}
                        />
                    ) : null}
                    <RangeField {...bb} field="textSize" label="文字字号" min={10} max={24} />
                    <ColorField {...bb} field="textColor" label="文字颜色" resetValue={defaultBuyButtonStyle.textColor} />
                    <ColorField {...bb} field="iconColor" label="图标颜色" resetValue={defaultBuyButtonStyle.iconColor} />
                    <RadioRowField
                        {...bb}
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
                                    <AlphaColorInput value={String(sub.gradientColorA ?? "")} onChange={(v) => bb.onChange({ gradientColorA: v })} />
                                </span>
                                <span style={{ margin: "0 6px", color: "#969799" }}>-</span>
                                <span className="dec-color-trigger">
                                    <AlphaColorInput value={String(sub.gradientColorB ?? "")} onChange={(v) => bb.onChange({ gradientColorB: v })} />
                                </span>
                            </div>
                        </div>
                    </SettingRow>
                    <RangeField {...bb} field="radius" label="按钮圆角" max={20} />
                    <RangeField {...bb} field="paddingTopBottom" label="按钮高度" max={10} />
                    {sub.text !== "" && sub.text !== undefined ? <RangeField {...bb} field="paddingLeftRight" label="按钮宽度" max={10} /> : null}
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
                        <GroupListEditor data={data} onChange={onChange} />
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
