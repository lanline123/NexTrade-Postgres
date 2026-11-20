/**
 * 商品详情设置（移植自 ref-editor mobileProductDetail/productDetailA{1,2,3}/Settings.vue）
 * 三风格共用本组件，差异经 props 传入：
 * - showPriceColor：A1 无「价格颜色」（仅划线价颜色），A2/A3 有
 * - buyBarSwapped：A2/A3 的购买栏样式值与文案互换（2=简约风 / 1=经典风），且按钮圆角在 style===2 时显示；A1 为 1=简约 / 2=经典、圆角在 style!==2 时显示
 * - showTools：A3 底部为两枚整宽按钮，无首页/客服/购物车显隐行
 * - showHeaderRow：A3 且海外版时显示「页面头部」组
 * 显隐类字段原版写 boolean（el-radio 1/0 → setVisibility 写 v===1），故用 SegmentedRadio 手工回写 boolean。
 */
import { useMemo } from "react";
import { ColorField, RangeField, SegmentedRadio, SettingGroup, SettingRow } from "../../../shared/settingWidgets";
import GroupListEditor from "../../../shared/GroupListEditor";
import { isMerchant, isOverseas } from "../../../contract/version";
import { useI18n } from "../../../../../i18n";
import { normalizePdSortList } from "./sortList";
import PreviewSamplePicker from "./PreviewSamplePicker";

interface Props {
    data: Record<string, any>;
    onChange: (patch: Record<string, any>) => void;
    showPriceColor?: boolean;
    buyBarSwapped?: boolean;
    showTools?: boolean;
    showHeaderRow?: boolean;
}

const VIS_OPTIONS = (t: (k: string) => string) => [
    { value: 1, label: t("显示") },
    { value: 0, label: t("隐藏") }
];

export default function ProductDetailSettings({ data, onChange, showPriceColor, buyBarSwapped, showTools = true, showHeaderRow }: Props) {
    const { t } = useI18n();
    const vis = (field: string) => (data[field] !== false ? 1 : 0);
    const setVis = (field: string) => (v: number | string) => onChange({ [field]: Number(v) === 1 });
    const showRadius = buyBarSwapped ? data.buyBarStyle === 2 : data.buyBarStyle !== 2;
    // 区块排序编辑喂归一后的生效列表（与前台消费口径镜像，见 ./sortList）：
    // 面板所见顺序/显隐即前台生效结果，任一编辑写回完整归一列表。
    const sortListData = useMemo(() => ({ moduleSortList: normalizePdSortList(data.moduleSortList) }), [data.moduleSortList]);
    return (
        <div className="h5-editor-settings-body">
            <SettingGroup title={t("预览样例")}>
                <PreviewSamplePicker />
            </SettingGroup>
            {showHeaderRow && isOverseas() ? (
                <SettingGroup title={t("页面头部")}>
                    <SettingRow label={t("是否显示头部")}>
                        <SegmentedRadio value={vis("showHeader")} onChange={setVis("showHeader")} options={VIS_OPTIONS(t)} />
                    </SettingRow>
                </SettingGroup>
            ) : null}
            <SettingGroup title={t("商品图片区")}>
                <SettingRow label={t("轮播图比例")}>
                    <SegmentedRadio
                        value={data.imageRatio}
                        onChange={(v) => onChange({ imageRatio: v })}
                        options={[
                            { value: "4:3", label: "4:3" },
                            { value: "1:1", label: "1:1" }
                        ]}
                    />
                </SettingRow>
                <SettingRow label={t("轮播图圆角")}>
                    <SegmentedRadio
                        value={data.imageRadiusStyle}
                        onChange={(v) => onChange({ imageRadiusStyle: v })}
                        options={[
                            { value: "square", label: t("直角") },
                            { value: "round", label: t("圆角") },
                            { value: "large", label: t("大圆角") }
                        ]}
                    />
                </SettingRow>
                <RangeField data={data} onChange={onChange} field="imageMargin" label={t("轮播图左右边距")} max={50} />
            </SettingGroup>
            <SettingGroup title={t("商品标题及价格")}>
                <SettingRow label={t("标题大小")}>
                    <SegmentedRadio
                        value={data.titleSize}
                        onChange={(v) => onChange({ titleSize: v })}
                        options={[
                            { value: "small", label: t("小") },
                            { value: "medium", label: t("中") },
                            { value: "large", label: t("大") }
                        ]}
                    />
                </SettingRow>
                <ColorField data={data} onChange={onChange} field="titleColor" label={t("标题颜色")} />
                <SettingRow label={t("最大展示行数")}>
                    <SegmentedRadio
                        value={data.titleMaxLines}
                        onChange={(v) => onChange({ titleMaxLines: Number(v) })}
                        options={[
                            { value: 1, label: t("一行") },
                            { value: 2, label: t("两行") },
                            { value: 3, label: t("三行") }
                        ]}
                    />
                </SettingRow>
                {showPriceColor ? <ColorField data={data} onChange={onChange} field="priceColor" label={t("价格颜色")} /> : null}
                <ColorField data={data} onChange={onChange} field="marketPriceColor" label={t("商品划线价颜色")} />
            </SettingGroup>
            {isMerchant() ? (
                <SettingGroup title={t("店铺设置")}>
                    <SettingRow label={t("显示店铺信息")}>
                        <SegmentedRadio value={vis("showShop")} onChange={setVis("showShop")} options={VIS_OPTIONS(t)} />
                    </SettingRow>
                </SettingGroup>
            ) : null}
            <SettingGroup title={t("购买按钮区")}>
                <SettingRow label={t("底部悬浮购买样式")}>
                    <SegmentedRadio
                        value={data.buyBarStyle}
                        onChange={(v) => onChange({ buyBarStyle: Number(v) })}
                        options={
                            buyBarSwapped
                                ? [
                                      { value: 2, label: t("简约风") },
                                      { value: 1, label: t("经典风") }
                                  ]
                                : [
                                      { value: 1, label: t("简约风") },
                                      { value: 2, label: t("经典风") }
                                  ]
                        }
                    />
                </SettingRow>
                {showRadius ? <RangeField data={data} onChange={onChange} field="buttonRadius" label={t("按钮圆角")} max={50} /> : null}
                {showTools ? (
                    <>
                        <SettingRow label={t("商城首页")}>
                            <SegmentedRadio value={vis("showHome")} onChange={setVis("showHome")} options={VIS_OPTIONS(t)} />
                        </SettingRow>
                        <SettingRow label={t("客服入口")}>
                            <SegmentedRadio value={vis("showService")} onChange={setVis("showService")} options={VIS_OPTIONS(t)} />
                        </SettingRow>
                        <SettingRow label={t("购物车")}>
                            <SegmentedRadio value={vis("showCart")} onChange={setVis("showCart")} options={VIS_OPTIONS(t)} />
                        </SettingRow>
                    </>
                ) : null}
            </SettingGroup>
            <SettingGroup title={t("区块排序与显隐")}>
                <GroupListEditor
                    data={sortListData}
                    onChange={(patch) => onChange({ moduleSortList: patch.moduleSortList })}
                    field="moduleSortList"
                    canAdd={false}
                    canRemove={false}
                    collapsible={false}
                    itemTitle={(item) => item.label || item.key}
                    renderItem={(item, _index, itemProps) => (
                        <div style={{ padding: "0 12px 6px" }}>
                            <SettingRow label={t("是否显示")}>
                                <SegmentedRadio
                                    value={item.visible !== false ? 1 : 0}
                                    onChange={(v) => itemProps.onChange({ visible: Number(v) === 1 })}
                                    options={VIS_OPTIONS(t)}
                                />
                            </SettingRow>
                        </div>
                    )}
                />
            </SettingGroup>
            <SettingGroup title={t("背景设置")}>
                <ColorField data={data} onChange={onChange} field="backgroundColor" label={t("背景颜色")} />
                <ColorField data={data} onChange={onChange} field="contentBackgroundColor" label={t("内容背景颜色")} />
            </SettingGroup>
        </div>
    );
}
