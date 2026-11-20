/**
 * 商品混排 预览（移植自 view/admin editor/modules/mobilePro/mixedProductDisplay/Preview.vue）
 *
 * 双列各 50% + 中缝 1px 分割线；每组商品 = 组内演示项（productId===0，getDemoList 口径）
 * + 按 pruductList 的 ids 拉取的真实商品（原版 watchEffect 语义）。
 * 原版 Swiper slidesPerView=2/spaceBetween=6 以横滑轨承接（项宽 50%）。
 * 右侧小图原版按原图宽高比 × 高 20px 计算宽度，本实现以 <img height=20> 等价呈现。
 * 标题/卖点颜色按原版 Preview 消费 moduleColor（审计 E 的「写死」条目属 mixedNotice，非本模块）。
 */
import { useMemo } from "react";
import { cssText, imageFormat, productImage } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import H5Countdown from "../../../shared/H5Countdown";
import { mixedProductDisplayDefaults } from "./index";
import "./Preview.css";

const picScaleMap: Record<string, string> = {
    "3:2": "calc(100% / 3 * 2)",
    "1:1": "calc(100% / 1 * 1)",
    "3:4": "calc(100% / 3 * 4)",
    "16:9": "calc(100% / 16 * 9)"
};

const formatMap: Record<string, string> = { 天: "DD", 时: "HH", 分: "mm", 秒: "ss", 毫秒: "SSS" };

function GroupColumn({
    module,
    group,
    fetched,
    isFirst
}: {
    module: Record<string, any>;
    group: Record<string, any>;
    fetched: Array<Record<string, any>>;
    isFirst: boolean;
}) {
    const demoList = (group?.pruductList ?? []).filter((item: any) => item.productId === 0);
    const list = [...demoList, ...fetched];
    const mc = module.moduleColor ?? {};
    const timeFormat = (module.timeContent ?? []).map((c: string) => formatMap[c]).filter(Boolean).join(":");

    return (
        <div className="mpd-cont">
            <div>
                <div className="mpd-mk-title" style={{ color: mc.titleColor }}>
                    {group?.groupTitle}
                    {isFirst && Number(module.showCountdown) === 1 ? (
                        <div
                            className="mpd-time"
                            style={cssText(`background:${mc.countdownBackgroundColor};`, `color:${mc.countdownColor};`)}
                        >
                            {Date.now() > Number((module.dailyTime ?? [])[0]) ? (
                                <H5Countdown
                                    countdownType={module.countdownType}
                                    valueStyle={{
                                        color: mc.countdownColor,
                                        fontSize: "12px",
                                        display: "block",
                                        height: "20px",
                                        lineHeight: "20px"
                                    }}
                                    format={timeFormat}
                                    dailyTime={module.dailyTime}
                                    specifyTime={module.specifyTime}
                                />
                            ) : (
                                "活动未开始"
                            )}
                        </div>
                    ) : null}
                    {!isFirst && module.rightPic?.picUrl ? (
                        <img
                            src={imageFormat(module.rightPic.picUrl)}
                            alt=""
                            style={{ marginLeft: 2, height: 20, width: "auto", display: "block" }}
                        />
                    ) : null}
                </div>
                <div className="mpd-mk-stitle" style={{ color: mc.sellingPointColor }}>
                    {group?.groupSubTitle}
                </div>
            </div>
            <div className="mpd-item-box">
                <div className="mpd-track">
                    {list.map((item: any, idx: number) => (
                        <div className="mpd-item" key={`${item.productId || 0}_${idx}`}>
                            <div
                                className="mpd-goods-img"
                                style={cssText(`padding-top:${picScaleMap[module.picScale] ?? picScaleMap["1:1"]};`, `border-radius:${module.productRadius}px;`)}
                            >
                                <div
                                    className="mpd-img"
                                    style={cssText(
                                        `padding-top:${picScaleMap[module.picScale] ?? picScaleMap["1:1"]};`,
                                        `border-radius:${module.productRadius}px;`,
                                        `background-size:${module.picFillType};`,
                                        `background-image:url(${productImage(item)});`
                                    )}
                                />
                            </div>
                            {Number(module.showProductPrice) === 1 ? (
                                <div className="mpd-price-box">
                                    <div
                                        className="mpd-price"
                                        style={cssText(
                                            `background:${mc.priceBackgroundColor};`,
                                            `color:${mc.priceColor};`,
                                            "font-weight:700;"
                                        )}
                                    >
                                        ￥{item.productPrice}
                                    </div>
                                </div>
                            ) : null}
                            {item.marketPrice && Number(module.crossedOutPrice) === 1 ? (
                                <div className="mpd-origin">
                                    <div style={cssText(`color:${mc.crossedOutPriceColor};`, "text-decoration:line-through;")}>
                                        {item.marketPrice}
                                    </div>
                                </div>
                            ) : null}
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}

export default function MixedProductDisplayPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, mixedProductDisplayDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const groups: Array<Record<string, any>> = module.productGroups ?? [];
    const idsOf = (g: Record<string, any>) => (g?.pruductList ?? []).map((item: any) => item.productId).filter(Boolean);
    // 两组各按 ids 拉取（原版逐组 getProductList({ids})）；组数固定为 2（原版模板写死 [0]/[1]）
    const fetched0 = useDecorateProductList({ productSelectType: idsOf(groups[0]).length ? 1 : 0, productIds: idsOf(groups[0]) });
    const fetched1 = useDecorateProductList({ productSelectType: idsOf(groups[1]).length ? 1 : 0, productIds: idsOf(groups[1]) });
    // 全空判定：两组均无标题文案、无演示项、无拉取结果时整块无任何可见内容，
    // 渲染标准空态占位（与同族模块 .module-ad-empty 口径一致），避免画布上只剩空壳列
    const groupHasContent = (g: Record<string, any> | undefined, fetched: Array<Record<string, any>>) =>
        !!g && (!!g.groupTitle || !!g.groupSubTitle || (g.pruductList ?? []).some((item: any) => item.productId === 0) || fetched.length > 0);
    const allEmpty = !groupHasContent(groups[0], fetched0) && !groupHasContent(groups[1], fetched1);

    return (
        <div
            className="module-ad-con module-mixed_product_display"
            style={cssText(
                commonStyle.moduleStyle.boxPaddingBottom,
                commonStyle.moduleStyle.boxPaddingTop,
                commonStyle.moduleStyle.boxPadding,
                commonStyle.moduleStyle.backgroundSize,
                commonStyle.moduleStyle.backgroundImage
            )}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    commonStyle.moduleContentStyle.backgroundImage,
                    commonStyle.moduleContentStyle.backgroundSize,
                    commonStyle.moduleContentStyle.boxRadius,
                    commonStyle.moduleContentStyle.boxShadow,
                    commonStyle.moduleContentStyle.innerPadding
                )}
            >
                {allEmpty ? (
                    <div className="module-ad-empty empty-image_ad">
                        <div className="image-empty-goods-bg" />
                        <div className="desc">点击添加商品</div>
                    </div>
                ) : (
                    <div className="mpd-comp">
                        <GroupColumn module={module} group={groups[0] ?? {}} fetched={fetched0} isFirst />
                        <div className="mpd-line-box">
                            <div className="mpd-line" style={cssText(`background:${(module.moduleColor ?? {}).lineColor};`)} />
                        </div>
                        <GroupColumn module={module} group={groups[1] ?? {}} fetched={fetched1} isFirst={false} />
                    </div>
                )}
            </div>
        </div>
    );
}
