/**
 * 商品滑动 设置（对齐 view/admin mobilePro/productSlide/Settings.vue）
 *
 * 页签：商品基本设置 / 扩展设置。
 * 逐卡标题/逐卡渐变按「选择商品」的商品个数（手选=ids 长度，分类/标签=productNumber）
 * 生成逐项编辑卡，写法与原版 proxy 语义一致（不足补兜底渐变色）；原版「展开/收起」
 * 切换为编辑器本地状态，本实现直接展开编辑（不写回模块数据）。
 * 原版在 productIds 变化时会按商品 id 重排逐卡标题/渐变（watch 重映射），本实现未移植
 * 该重映射逻辑——调换商品顺序后逐卡标题/渐变按位置保留（已在小结注明）。
 * 模块设置段的 frame/boxRadius/innerBgColor/moduleBgImage 为本模块自有结构，
 * 非通用 ExtensionStyleSettings（原版无 type 门控页签项）。
 */
import { Input } from "@douyinfe/semi-ui";
import AlphaColorInput from "../../../../../components/AlphaColorInput";
import {
    SettingGroup,
    SettingRow,
    RadioRowField,
    RadioField,
    RangeField,
    ColorField,
    PicSelectField,
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import ProductsSourceField from "../../../shared/ProductsSourceField";
import { defaultProductSlide } from "../../../contract/sharedDefaults";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

const GRADIENT_FALLBACK: Array<[string, string]> = [
    ["#B6A8F5", "#8E7BF0"],
    ["#FF8FB8", "#FF5E96"],
    ["#FFD98A", "#FBC25A"],
    ["#C7BEF8", "#9C8CF0"]
];

const COLOR_ROWS: Array<{ key: string; label: string; reset: string }> = [
    { key: "cardTitleColor", label: "商品标题色", reset: defaultProductSlide.cardTitleColor },
    { key: "briefColor", label: "商品卖点色", reset: defaultProductSlide.briefColor },
    { key: "descColor", label: "商品描述文字色", reset: defaultProductSlide.descColor },
    { key: "productPriceColor", label: "商品价格色", reset: defaultProductSlide.productPriceColor },
    { key: "productMarketPriceColor", label: "商品原价色", reset: defaultProductSlide.productMarketPriceColor },
    { key: "productSalesColor", label: "商品销量色", reset: defaultProductSlide.productSalesColor },
    { key: "badgeColor", label: "角标色", reset: defaultProductSlide.badgeColor },
    { key: "buyBtnTextColor", label: "按钮文字色", reset: defaultProductSlide.buyBtnTextColor }
];

function productCountOf(data: Record<string, any>): number {
    const p = data?.products;
    if (!p) return 0;
    if (Number(p.productSelectType) === 1) return Array.isArray(p.productIds) ? p.productIds.length : 0;
    return Number(p.productNumber) || 0;
}

function GradientPair({ start, end, onStart, onEnd }: { start: string; end: string; onStart: (v: string) => void; onEnd: (v: string) => void }) {
    return (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
            <AlphaColorInput value={start ?? ""} onChange={onStart} />
            <span style={{ color: "#999" }}>~</span>
            <AlphaColorInput value={end ?? ""} onChange={onEnd} />
        </span>
    );
}

export default function ProductSlideSettings({ data, onChange }: Props) {
    const count = productCountOf(data);
    const isDynamic = [2, 3].includes(Number(data?.products?.productSelectType));
    const titles: string[] = Array.isArray(data?.cardTitleList) ? data.cardTitleList : [];
    const starts: string[] = Array.isArray(data?.cardGradientStartList) ? data.cardGradientStartList : [];
    const ends: string[] = Array.isArray(data?.cardGradientEndList) ? data.cardGradientEndList : [];
    const setListAt = (field: string, list: string[], index: number, value: string) => {
        const next = [...list];
        while (next.length <= index) next.push("");
        next[index] = value;
        onChange({ [field]: next });
    };
    const frame = data?.frame && typeof data.frame === "object" ? data.frame : {};
    const frameProps: Props = { data: frame, onChange: (patch: Patch) => onChange({ frame: { ...frame, ...patch } }) };

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["商品基本设置", "扩展设置"]}>
                <>
                    <SettingGroup title="选择商品">
                        <ProductsSourceField data={data} onChange={onChange} />
                    </SettingGroup>
                    <SettingGroup title="商品卖点描述设置">
                        <SettingRow label="逐卡标题文字" block>
                            {count === 0 ? (
                                <div style={{ color: "#646a73", fontSize: 12 }}>请先在上方「选择商品」中添加商品后再来配置逐卡标题</div>
                            ) : (
                                <div>
                                    {Array.from({ length: count }, (_, index) => (
                                        <div key={index} style={{ marginBottom: 8 }}>
                                            <div style={{ fontSize: 12, color: "#1c1f23", marginBottom: 4 }}>
                                                第 {index + 1} 个商品
                                                {isDynamic ? <span style={{ color: "#999", marginLeft: 6 }}>动态拉取</span> : null}
                                            </div>
                                            <Input
                                                value={titles[index] ?? ""}
                                                maxLength={8}
                                                placeholder="如：服饰专场"
                                                style={{ width: 240, maxWidth: "100%" }}
                                                onChange={(v: string) => setListAt("cardTitleList", titles, index, v)}
                                            />
                                        </div>
                                    ))}
                                </div>
                            )}
                        </SettingRow>
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="展示模式">
                        <SettingRow label="商品层叠样式" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="cardStyle"
                                options={[
                                    { value: "noBorderWhite", label: "无边白底" },
                                    { value: "shadow", label: "卡片投影" },
                                    { value: "border", label: "描边白底" },
                                    { value: "transparent", label: "无边透底" }
                                ]}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="productPicFillType"
                            label="图片填充"
                            options={[
                                { value: "cover", label: "充满" },
                                { value: "contain", label: "周边留白" }
                            ]}
                        />
                        <RangeField data={data} onChange={onChange} field="productImgRadius" label="商品底角" max={30} />
                        <RadioRowField data={data} onChange={onChange} field="showImagePlatform" label="图片下平台" options={SHOW_OPTIONS} />
                        <SettingRow label="图片比例" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="productPicRatio"
                                options={[
                                    { value: "default", label: "默认" },
                                    { value: "1:1", label: "1:1" },
                                    { value: "3:4", label: "3:4" },
                                    { value: "3:2", label: "3:2" }
                                ]}
                            />
                        </SettingRow>
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="textWeight"
                            label="文本样式"
                            options={[
                                { value: 400, label: "常规体" },
                                { value: 700, label: "加粗体" }
                            ]}
                        />
                        <SettingRow label="文本对齐" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="textAlign"
                                options={[
                                    { value: "left", label: "左对齐" },
                                    { value: "center", label: "居中" },
                                    { value: "right", label: "右对齐" }
                                ]}
                            />
                        </SettingRow>
                        <RangeField data={data} onChange={onChange} field="layerGap" label="商品层间距" max={30} />
                        <RangeField data={data} onChange={onChange} field="productItemGap" label="商品间间距" max={30} />
                        <RangeField data={data} onChange={onChange} field="cardRadius" label="卡片圆角" max={40} />
                        <RangeField data={data} onChange={onChange} field="productPercent" label="单卡宽度" min={30} max={60} />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="显示内容">
                        <RadioRowField data={data} onChange={onChange} field="showProductName" label="商品名称" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showFeature" label="商品特征" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showBrief" label="商品卖点" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showPrice" label="商品价格" options={SHOW_OPTIONS} />
                        {Number(data?.showPrice) === 1 ? (
                            <>
                                <RangeField data={data} onChange={onChange} field="priceFontSize" label="价格字号" min={10} max={24} />
                                <RadioRowField
                                    data={data}
                                    onChange={onChange}
                                    field="priceBold"
                                    label="价格粗体"
                                    options={[
                                        { value: 1, label: "粗体" },
                                        { value: 0, label: "常规" }
                                    ]}
                                />
                            </>
                        ) : null}
                        <RadioRowField data={data} onChange={onChange} field="showMarketPrice" label="商品原价" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showSales" label="商品销量" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showSlidePeek" label="划入查看" options={SHOW_OPTIONS} />
                        <SettingRow label="购买按钮样式" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="buyBtnStyle"
                                options={[
                                    { value: 0, label: "不显示" },
                                    { value: 1, label: "样式一" },
                                    { value: 2, label: "样式二" },
                                    { value: 3, label: "样式三" },
                                    { value: 4, label: "样式四" },
                                    { value: 5, label: "样式五" },
                                    { value: 6, label: "样式六" },
                                    { value: 7, label: "样式七" },
                                    { value: 8, label: "样式八" }
                                ]}
                            />
                        </SettingRow>
                        <SettingRow label="商详跳转模式" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="detailJumpMode"
                                options={[
                                    { value: "default", label: "默认" },
                                    { value: "native", label: "原生" }
                                ]}
                            />
                        </SettingRow>
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="颜色设置">
                        <SettingRow label="逐卡背景渐变" block>
                            {count === 0 ? (
                                <div style={{ color: "#646a73", fontSize: 12 }}>请先在「商品基本设置」中添加商品后再来配置逐卡渐变</div>
                            ) : (
                                <div>
                                    {Array.from({ length: count }, (_, index) => (
                                        <div key={index} style={{ marginBottom: 10 }}>
                                            <div style={{ fontSize: 12, color: "#1c1f23", marginBottom: 4 }}>第 {index + 1} 个商品</div>
                                            <GradientPair
                                                start={starts[index] || GRADIENT_FALLBACK[index % GRADIENT_FALLBACK.length][0]}
                                                end={ends[index] || GRADIENT_FALLBACK[index % GRADIENT_FALLBACK.length][1]}
                                                onStart={(v) => setListAt("cardGradientStartList", starts, index, v)}
                                                onEnd={(v) => setListAt("cardGradientEndList", ends, index, v)}
                                            />
                                        </div>
                                    ))}
                                </div>
                            )}
                        </SettingRow>
                    </SettingGroup>
                    <DecDivider />
                    {COLOR_ROWS.map((row) => (
                        <ColorField key={row.key} data={data} onChange={onChange} field={row.key} label={row.label} resetValue={row.reset} />
                    ))}
                    <SettingRow label="按钮背景渐变">
                        <GradientPair
                            start={data?.buyBtnGradientStart ?? defaultProductSlide.buyBtnGradientStart}
                            end={data?.buyBtnGradientEnd ?? defaultProductSlide.buyBtnGradientEnd}
                            onStart={(v) => onChange({ buyBtnGradientStart: v })}
                            onEnd={(v) => onChange({ buyBtnGradientEnd: v })}
                        />
                    </SettingRow>
                    <DecDivider />
                    <SettingGroup title="模块设置">
                        <RangeField {...frameProps} field="boxPadding" label="模块左右边距" max={30} />
                        <RangeField {...frameProps} field="boxPaddingTop" label="模块上边距" max={30} />
                        <RangeField {...frameProps} field="boxPaddingBottom" label="模块下边距" max={30} />
                        <RangeField {...frameProps} field="boxRadius" label="卡片区圆角" max={40} />
                        <ColorField data={data} onChange={onChange} field="innerBgColor" label="模块内部背景色" resetValue={defaultProductSlide.innerBgColor} />
                        <ColorField {...frameProps} field="backgroundColor" label="模块背景色" resetValue="" />
                        <PicSelectField data={data} onChange={onChange} field="moduleBgImage" label="模块背景图：宽度750px，高度不限" />
                    </SettingGroup>
                </>
            </SettingsTabs>
        </div>
    );
}
