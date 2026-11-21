/**
 * 商品轮播 设置（对齐 view/admin mobilePro/productCarousel/Settings.vue）
 *
 * 页签：商品设置 / 扩展设置（颜色设置 7 色 + 模块参数 + 背景图/模式）。
 * 选择商品走共享 ProductInfoSelect（快照式选品）：可增删/更换/拖拽排序，
 * 选择后按原版口径拉取最新商品信息写入 productList 快照。
 * BuyBtnEdit 按 buyBtn/Edit.vue 字段集本地实现（buyBtnStyle 对象，历史数字值回退默认对象）。
 * 标题文字 el-input maxlength 20 以块级输入承接。
 */
import { Input } from "@douyinfe/semi-ui";
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
    DecDivider
} from "../../../shared/settingWidgets";
import { SettingsTabs } from "../../../shared/styleEdits";
import { cloneDeep, defaultProductCarousel as defaults } from "../../../contract/sharedDefaults";
import { resolveObjBuyButton } from "../../../../components/buyStyleObj";
import { usePcGlobalBuyStyle } from "../../../decoratePc/previews/shared/PcBuyButton";
import BuyBtnFollowRow from "../../../shared/BuyBtnFollowRow";
import ProductInfoSelect from "../../../shared/ProductInfoSelect";
import { productCarouselDefaults } from "./index";
import { useI18n } from "../../../../../i18n";

type Patch = Record<string, any>;
interface Props {
    data: Record<string, any>;
    onChange: (patch: Patch) => void;
}

const SHOW_OPTIONS = [
    { value: 1, label: "显示" },
    { value: 0, label: "不显示" }
];

function BuyBtnFields({ data, onChange }: Props) {
    const btn = data?.buyBtnStyle && typeof data.buyBtnStyle === "object" ? data.buyBtnStyle : cloneDeep(defaults.buyBtnStyle);
    // 跟随全局：开启时下方字段显示全局当前值并置灰只读；写回永远合并进 btn
    // （模块原值），全局值绝不写进模块数据——切回自定义即恢复模块原设置。
    const global = usePcGlobalBuyStyle();
    const following = Number(btn.followGlobal) === 1;
    const effBtn = following ? resolveObjBuyButton(btn, global) : btn;
    const sub: Props = { data: effBtn, onChange: (patch: Patch) => onChange({ buyBtnStyle: { ...btn, ...patch } }) };
    return (
        <>
            <RadioRowField
                {...sub}
                field="showBtn"
                label="购买按钮"
                options={[
                    { value: 0, label: "不显示" },
                    { value: 1, label: "显示" }
                ]}
            />
            {Number(btn.showBtn) === 1 ? (
                <>
                    <BuyBtnFollowRow data={btn} onChange={(patch: Patch) => onChange({ buyBtnStyle: { ...btn, ...patch } })} />
                    <div style={following ? { opacity: 0.55, pointerEvents: "none" } : undefined}>
                    <SettingRow label="购买按钮图标样式" block>
                        <BuyBtnStyleField {...sub} field="btnType" visualSet="h5BtnType" customIcon={effBtn.iconImage} />
                    </SettingRow>
                    {Number(effBtn.btnType) === 5 ? <PicSelectField {...sub} field="iconImage" label="自定义图标图片" /> : null}
                    <RangeField {...sub} field="iconSize" label="图标尺寸" min={12} max={40} />
                    <TextField {...sub} field="text" label="文字内容" placeholder="请输入按钮文字" />
                    </div>
                    <RadioRowField
                        {...sub}
                        field="wrap"
                        label="按钮换行"
                        options={[
                            { value: 0, label: "不换行" },
                            { value: 1, label: "换行" }
                        ]}
                    />
                    {Number(btn.wrap) === 1 ? (
                        <RadioRowField
                            {...sub}
                            field="wrapAlign"
                            label="换行后对齐"
                            options={[
                                { value: 1, label: "居左" },
                                { value: 2, label: "居中" },
                                { value: 3, label: "居右" }
                            ]}
                        />
                    ) : null}
                    <div style={following ? { opacity: 0.55, pointerEvents: "none" } : undefined}>
                    <RangeField {...sub} field="textSize" label="文字字号" min={10} max={24} />
                    <ColorField {...sub} field="textColor" label="文字颜色" resetValue="#ffffff" />
                    <ColorField {...sub} field="iconColor" label="图标颜色" resetValue="#ffffff" />
                    <RadioRowField
                        {...sub}
                        field="gradientType"
                        label="背景渐变方向"
                        options={[
                            { value: "upDown", label: "上下" },
                            { value: "leftRight", label: "左右" },
                            { value: "diagonal", label: "斜向" },
                            { value: "purity", label: "纯色" }
                        ]}
                    />
                    <ColorField {...sub} field="gradientColorA" label="背景渐变颜色A" resetValue="#333333" />
                    <ColorField {...sub} field="gradientColorB" label="背景渐变颜色B" resetValue="#333333" />
                    <RangeField {...sub} field="radius" label="按钮圆角" max={20} />
                    <RangeField {...sub} field="paddingTopBottom" label="按钮高度" max={10} />
                    <RangeField {...sub} field="paddingLeftRight" label="按钮宽度" max={10} />
                    </div>
                </>
            ) : null}
        </>
    );
}

export default function ProductCarouselSettings({ data, onChange }: Props) {
    const { t } = useI18n();
    const productList: Array<Record<string, any>> = Array.isArray(data?.productList) ? data.productList : [];
    const frame = data?.frame ?? {};
    const frameProps: Props = { data: frame, onChange: (patch: Patch) => onChange({ frame: { ...frame, ...patch } }) };

    const colorRows: Array<{ field: string; label: string }> = [
        { field: "moduleTitleColor", label: "模块标题色" },
        { field: "productTitleColor", label: "商品标题色" },
        { field: "descColor", label: "商品描述文字色" },
        { field: "priceColor", label: "商品价格色" },
        { field: "marketPriceColor", label: "商品原价色" },
        { field: "salesColor", label: "商品销量色" },
        { field: "badgeColor", label: "角标色" }
    ];

    return (
        <div className="h5-editor-settings-body">
            <SettingsTabs tabs={["商品设置", "扩展设置"]}>
                <>
                    <SettingGroup title="选择商品">
                        <div style={{ padding: "0 16px 10px" }}>
                            <ProductInfoSelect value={productList} onChange={(next) => onChange({ productList: next })} isMultiple />
                            <div style={{ color: "#646a73", fontSize: 12, lineHeight: 1.6, marginTop: 8 }}>
                                提示：选择商品后自动加载商品信息；可通过拖拽进行商品排序
                            </div>
                        </div>
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="参数设置">
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="showModuleTitle"
                            label="模块标题"
                            options={[
                                { value: 1, label: "显示" },
                                { value: 0, label: "隐藏" }
                            ]}
                        />
                        {Number(data?.showModuleTitle) === 1 ? (
                            <SettingRow label="标题文字" block>
                                <Input value={data?.moduleTitle ?? ""} placeholder={t("请输入模块标题")} maxLength={20} showClear style={{ width: "100%" }} onChange={(v: string) => onChange({ moduleTitle: v })} />
                            </SettingRow>
                        ) : null}
                        <SettingRow label="商品展示样式" block>
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
                        <RangeField data={data} onChange={onChange} field="goodsRadius" label="商品圆角" max={30} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="picBottomRadius"
                            label="图片下圆角"
                            options={[
                                { value: 1, label: "圆角" },
                                { value: 0, label: "直角" }
                            ]}
                        />
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
                        <RangeField data={data} onChange={onChange} field="itemGap" label="商品外间距" max={30} />
                        <RangeField data={data} onChange={onChange} field="innerPadding" label="商品内间距" max={30} />
                        <RangeField data={data} onChange={onChange} field="cardWidthPercent" label="单卡宽度" min={60} max={95} />
                        <RadioRowField
                            data={data}
                            onChange={onChange}
                            field="autoplay"
                            label="自动播放"
                            options={[
                                { value: 1, label: "开" },
                                { value: 0, label: "关" }
                            ]}
                        />
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="显示内容">
                        <RadioRowField data={data} onChange={onChange} field="showName" label="商品名称" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showDesc" label="商品描述" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showPrice" label="商品价格" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showMarketPrice" label="商品原价" options={SHOW_OPTIONS} />
                        <RadioRowField data={data} onChange={onChange} field="showSales" label="商品销量" options={SHOW_OPTIONS} />
                        <BuyBtnFields data={data} onChange={onChange} />
                    </SettingGroup>
                </>
                <>
                    <SettingGroup title="颜色设置">
                        {colorRows.map((row) => (
                            <ColorField key={row.field} data={data} onChange={onChange} field={row.field} label={row.label} resetValue={(productCarouselDefaults as any)[row.field] ?? ""} />
                        ))}
                    </SettingGroup>
                    <DecDivider />
                    <SettingGroup title="模块参数">
                        <RangeField {...frameProps} field="boxPadding" label="模块左右边距" max={30} />
                        <RangeField {...frameProps} field="boxPaddingTop" label="模块上边距" max={30} />
                        <RangeField {...frameProps} field="boxPaddingBottom" label="模块下边距" max={30} />
                        <RangeField data={data} onChange={onChange} field="titleContentGap" label="标题与内容间距" max={40} />
                        <ColorField {...frameProps} field="backgroundColor" label="模块背景色" resetValue="" />
                        <PicSelectField data={data} onChange={onChange} field="moduleBgImage" label="模块背景图：宽度750px，高度不限" />
                        <SettingRow label="背景图模式" block>
                            <RadioField
                                data={data}
                                onChange={onChange}
                                field="bgImageMode"
                                options={[
                                    { value: "normal", label: "常规充满" },
                                    { value: "repeat", label: "平铺" },
                                    { value: "fixed", label: "固定" }
                                ]}
                            />
                        </SettingRow>
                    </SettingGroup>
                </>
            </SettingsTabs>
        </div>
    );
}
