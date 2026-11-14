/**
 * 秒杀（样式2） 预览（DOM 类名逐字对齐 uniapp-react 渲染端 seckill2/index.tsx + index.scss）
 *
 * 与 seckill 的差异：标题右侧带「距结束」倒计时（countdown-box/countdown-item）、
 * 商品卡带优惠百分比 tips、style=10 时横向滑动且显示划线原价（price-through）。
 * 商品在渲染端由 getHomeSeckill 接口拉取，预览用演示商品兜底；演示字段与渲染端
 * 使用一致：productId、productName、seckillPrice、marketPrice、orgProductPrice、
 * picUrl、seckkillData.seckillEndTime（渲染端字段名原文，含拼写）。
 * 倒计时用 shared/H5Countdown（specifyTime=首个商品的秒杀结束时间戳）。
 */
import { useMemo } from "react";
import CommonTitle from "../../../shared/CommonTitle";
import H5Countdown from "../../../shared/H5Countdown";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { seckill2Defaults } from "./index";
import "./Preview.css";

const DEMO_PICS = [
    "https://oss.tigshop.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071axvkfUDkQNFbIzF1iY.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071lOa0ZBx2mYlvq1lgjr.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071xUnObq2LokKPFecwXq.jpg"
];

/** 与渲染端 getDiscount 一致：优惠百分比（向下取整到 0.1） */
function getDiscount(currentPrice: number, originalPrice: number): number {
    if (originalPrice === 0 || currentPrice >= originalPrice) return 0;
    return Math.floor(((originalPrice - currentPrice) / originalPrice) * 1000) / 10;
}

export default function Seckill2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, seckill2Defaults);
        return merged;
    }, [data]);

    // 演示商品（仅预览兜底）
    const demoList = useMemo(
        () =>
            DEMO_PICS.map((pic, i) => ({
                productId: i + 1,
                productName: "演示秒杀商品（编辑模块后由接口加载真实秒杀商品）",
                seckillPrice: 99 + i * 100,
                marketPrice: 199 + i * 100,
                orgProductPrice: 199 + i * 100,
                picUrl: pic,
                picThumb: pic,
                seckkillData: { seckillEndTime: new Date(Date.now() + (i + 2) * 3600000).toISOString() }
            })),
        []
    );
    const endTimestamp = useMemo(() => (demoList[0] ? new Date(demoList[0].seckkillData.seckillEndTime).getTime() : 0), [demoList]);

    const frameFormat = formatFrame(module.frame ?? {});
    const allFormat = {
        backgroundColor: module.backgroundColor ? `background-color:${module.backgroundColor};` : "",
        boxPadding: `margin-left:${module.boxPadding ?? 0}px;margin-right:${module.boxPadding ?? 0}px;`,
        boxPaddingBottom: `margin-bottom:${module.boxPaddingBottom ?? 0}px;`,
        boxPaddingTop: `margin-top:${module.boxPaddingTop ?? 0}px;`,
        boxRadius: `border-radius:${module.boxRadius ?? 0}px;`,
        innerPadding: `padding:${module.innerPadding ?? 0}px;`
    };
    const isStyle10 = Number(module.style) === 10;
    const itemContentStyle = `${isStyle10 ? "padding-left" : "padding"}: ${module.goodsPadding ?? 0}px;`;
    const goodsConStyle = isStyle10 ? "display:flex;overflow:hidden;overflow-x:auto;flex-wrap:nowrap;padding-bottom:10px;" : "";
    const adItemStyle: React.CSSProperties = isStyle10 ? { width: `${100 / 3}%`, flexShrink: 0, flexGrow: 0 } : {};

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad module-seckill2 ad-style__" +
                module.style +
                " ad-buy_btn_style__" +
                module.buyBtnStyle +
                " ad-goods_style__" +
                module.goodsStyle +
                " ad-goods_radio_style__" +
                module.goodsRadioStyle +
                " ad-text_align__" +
                module.textAlign +
                " ad-text_weight__" +
                module.textWeight +
                " ad-goods_name_row__" +
                module.goodsNameRow +
                " ad-goods_name_padding__" +
                module.goodsNamePadding
            }
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom, allFormat.backgroundColor)}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    frameFormat.backgroundColor,
                    frameFormat.boxRadius,
                    allFormat.boxPadding,
                    allFormat.boxPaddingBottom,
                    allFormat.boxPaddingTop,
                    allFormat.boxRadius,
                    allFormat.innerPadding
                )}
            >
                {module.title?.showTitle === 1 || module.title?.showTitle === "1" ? (
                    <>
                        <CommonTitle data={module.title} />
                        <H5Countdown countdownType="specifyTime" specifyTime={endTimestamp} format="DD HH:mm:ss">
                            {(timeData) => (
                                <div className="countdown-box">
                                    <span className="countdown-text">距结束</span>
                                    {Number(timeData.days) > 0 && (
                                        <>
                                            <span className={`countdown-item ${String(timeData.days).length > 2 ? "days" : ""}`}>{timeData.days}</span>
                                            <span className="countdown-separator-text">天</span>
                                        </>
                                    )}
                                    <span className="countdown-item">{timeData.hours}</span>
                                    <span className="countdown-separator">:</span>
                                    <span className="countdown-item">{timeData.minutes}</span>
                                    <span className="countdown-separator">:</span>
                                    <span className="countdown-item">{timeData.seconds}</span>
                                    <span className="iconfont-h5 icon-youjiantou" />
                                </div>
                            )}
                        </H5Countdown>
                    </>
                ) : null}
                <div className="goods-ad-warp">
                    <div className="goods-ad-con" style={cssText(goodsConStyle)}>
                        {demoList.map((item) => (
                            <div key={item.productId} className="goods-ad-item" style={adItemStyle}>
                                <div className="item-content" style={cssText(itemContentStyle)}>
                                    <a className="item-image-a">
                                        <div className="item-con">
                                            <div className="item-photo">
                                                <img src={productImage(item)} alt="" />
                                                <div className="item-photo-bg" style={{ backgroundColor: module.imageBackgroundColor }} />
                                            </div>
                                            <div className="item-info">
                                                {Number(module.showName) === 1 ? (
                                                    <div className="item-name">
                                                        <a className="item-name-a">{item.productName}</a>
                                                    </div>
                                                ) : null}
                                                <div className="tips">优惠{getDiscount(Number(item.seckillPrice), Number(item.marketPrice))}%</div>
                                                {module.showPrice ? (
                                                    <div className="item-action">
                                                        <div className="item-price">
                                                            <b className="price_format">{priceFormat(item.seckillPrice)}</b>
                                                            {isStyle10 ? (
                                                                <div className="price-through">
                                                                    <span style={{ textDecoration: "line-through" }}>{priceFormat(item.orgProductPrice)}</span>
                                                                </div>
                                                            ) : null}
                                                        </div>
                                                    </div>
                                                ) : null}
                                            </div>
                                        </div>
                                    </a>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
