/**
 * 分组滑动 预览（移植自 view/admin editor/modules/mobilePro/groupSliding/Preview.vue）
 *
 * 组卡横滑（宽 groupWidth%、圆角 groupRadius、内距 10px 8px 8px）：组题（titlePic 图标高 20 +
 * 大标题 titleSize/600 + 副标题）、商品白底盒内逐行商品（图宽/高 = productPicRatio%，
 * 前三名次徽标 ranking 图 20px，商品名 14px/18px 行高钳制，标签行，价格 + 划线价 + 购买按钮）。
 * 商品直接用各组 productList 快照渲染（原版 productId=0 演示项走 getDemoList 同口径）。
 * 注：原版 Preview 中商品名颜色写死 rgb(51,51,51)、价格粗细写死 700（未消费 moduleColor.productNameColor
 * 与 productPrice_weight/_size），此处以源码为准；名次徽标为原版 3 张 OSS 图。
 */
import { useMemo } from "react";
import { cssText, imageFormat, productImage } from "../../../shared/format";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import BuyButton from "../../../shared/BuyButton";
import { groupSlidingDefaults } from "./index";
import "./Preview.css";

const RANKING_IMGS = [
    "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732094008g4ZMmuQbNcBlYrNCuz.png",
    "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732094013OJOLUek96BizcTvwmU.png",
    "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17320940168dzp9YD5LbpcMMjqe9.png"
];

export default function GroupSlidingPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, groupSlidingDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const groupList: any[] = Array.isArray(module.groupList) ? module.groupList : [];
    const moduleColor = module.moduleColor || {};
    const picRadiusStyle = module.productPicRightRadius
        ? `${module.productPicRadius}px`
        : `${module.productPicRadius}px 0 0 ${module.productPicRadius}px`;
    const formatLabel = (label: string) => String(label ?? "").split("\n");

    return (
        <div
            className="module-ad-con h5-group-sliding"
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
                <div>
                    <div className="groupsliding">
                        {groupList.map((group, index) => (
                            <div
                                className="slide-box"
                                key={index}
                                style={{ backgroundColor: group.backgroundColor, borderRadius: `${module.groupRadius}px`, width: `${module.groupWidth}%`, flexShrink: 0 }}
                            >
                                <div className="bg_title">
                                    <div className="title_box">
                                        {moduleColor.titlePic?.picUrl ? <img className="img" src={imageFormat(moduleColor.titlePic.picUrl)} alt="" /> : null}
                                        <div className="p" style={{ color: moduleColor.titleColor, fontSize: `${moduleColor.titleSize}px` }}>
                                            {group.title}
                                        </div>
                                    </div>
                                    <div className="p" style={{ color: moduleColor.subTitleColor, fontSize: `${moduleColor.subTitleSize}px` }}>
                                        {group.subTitle}
                                    </div>
                                </div>
                                <div className="item-box" style={{ borderRadius: `${module.groupRadius}px`, background: moduleColor.productBackgroundColor }}>
                                    {(group.productList || []).map((item: any, itemIndex: number) => (
                                        <div className="li" key={itemIndex}>
                                            <div className="goods-item" style={{ display: "flex", borderRadius: picRadiusStyle }}>
                                                <div
                                                    className="goods-img"
                                                    style={{ flexShrink: 0, width: `${module.productPicRatio}%`, paddingTop: `${module.productPicRatio}%` }}
                                                >
                                                    <div
                                                        className="img"
                                                        style={{ borderRadius: picRadiusStyle, backgroundSize: "cover", backgroundImage: `url(${productImage(item)})` }}
                                                    />
                                                    {itemIndex <= 2 && Number(module.showRanking) === 1 ? (
                                                        <img className="ranking" src={RANKING_IMGS[itemIndex]} alt="" />
                                                    ) : null}
                                                </div>
                                                <div className="goods-info" style={{ justifyContent: "center", padding: "0px 0px 0px 10px" }}>
                                                    {Number(module.showProductName) === 1 ? (
                                                        <div
                                                            className="goods-title"
                                                            style={{
                                                                marginTop: 0,
                                                                height: `${Number(module.productNameRow) * 18}px`,
                                                                lineHeight: "18px",
                                                                WebkitLineClamp: module.productNameRow,
                                                                fontWeight: module.productNameWeight,
                                                                color: "rgb(51, 51, 51)",
                                                                fontSize: `${module.productNameSize}px`
                                                            }}
                                                        >
                                                            {item.productName}
                                                        </div>
                                                    ) : null}
                                                    {Number(module.showProductLabel) === 1 && formatLabel(group.label)[itemIndex] ? (
                                                        <div className="tag">
                                                            <div
                                                                className="span"
                                                                style={{
                                                                    padding: "0px 5px",
                                                                    backgroundColor: moduleColor.labelBackgroundColor,
                                                                    color: moduleColor.labelTextColor,
                                                                    display: "inline-block"
                                                                }}
                                                            >
                                                                {formatLabel(group.label)[itemIndex]}
                                                            </div>
                                                        </div>
                                                    ) : null}
                                                    <div className="goods-price-box" style={{ marginTop: `${module.productPriceMarginTop}px` }}>
                                                        <div className="price-origin-sale">
                                                            <div className="price-origin-icon">
                                                                <div className="price-icon">
                                                                    {Number(module.showProductPrice) === 1 ? (
                                                                        <div className="price" style={{ color: moduleColor.productPriceColor, fontWeight: 700 }}>
                                                                            ￥{item.productPrice}
                                                                        </div>
                                                                    ) : null}
                                                                </div>
                                                                {Number(module.crossedOutPrice) === 1 ? (
                                                                    <div className="origin" style={{ color: moduleColor.crossedOutPriceColor }}>
                                                                        ¥{item.marketPrice}
                                                                    </div>
                                                                ) : null}
                                                            </div>
                                                        </div>
                                                        {module.buyBtnStyle?.showBtn ? <BuyButton styleOptions={module.buyBtnStyle} /> : null}
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
