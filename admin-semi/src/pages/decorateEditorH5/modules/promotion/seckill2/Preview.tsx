/**
 * 秒杀2 预览（移植自 view/admin editor/modules/mobilePromotion/seckill2/Preview.vue）
 *
 * 无 frame（defaults 契约）：外层/内容层样式全部由顶层字段拼（原版 formatFrame({}) 的串为无效 CSS 被浏览器丢弃，
 * 结果等价）。标题区为 CommonTitle + #more 槽倒计时（距结束 + 黑底白字时间块，取首条活动结束时间，
 * 以 H5Countdown specifyTime 承接）；商品同 seckill 取数（/promotion/seckill/listForHomePreview）。
 * 卡片：item-photo-bg 垫图背景色 imageBackgroundColor、优惠 tips 红 chip（getDiscount 逐字复刻，
 * orgProductPrice 缺失时回退 marketPrice/现价→0，见摘要）、style 10 横滑（一行 3 个、item-content 仅左内边距、
 * 价格 16px #1a1a1a + 划线原价）。
 */
import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { apiGet } from "../../../../../api";
import H5Countdown from "../../../shared/H5Countdown";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { formatTitle, mergeDefaultModule } from "../../../contract/sharedDefaults";
import { seckill2Defaults } from "./index";
import "./Preview.css";

/** 折扣百分比（逐字复刻原版 getDiscount） */
function getDiscount(currentPrice: number, originalPrice: number) {
    if (originalPrice === 0 || currentPrice >= originalPrice) return 0;
    const discountPercentage = ((originalPrice - currentPrice) / originalPrice) * 100;
    return Math.floor(discountPercentage * 10) / 10;
}

function dateStringToTimestamp(dateString?: string): number {
    if (!dateString) return 0;
    const parts = String(dateString).split(" ");
    const [year, month, day] = (parts[0] || "").split("-");
    const [hour, minute, second] = (parts[1] || "0:0:0").split(":");
    return new Date(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second)).getTime();
}

export default function Seckill2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, seckill2Defaults);
        return merged;
    }, [data]);

    const [list, setList] = useState<any[]>([]);
    useEffect(() => {
        let alive = true;
        apiGet<any>("/promotion/seckill/listForHomePreview", { page: 1, size: 15 })
            .then((res: any) => {
                if (alive) setList(res?.records || res?.list || (Array.isArray(res) ? res : []));
            })
            .catch(() => undefined);
        return () => {
            alive = false;
        };
    }, []);

    const style = Number(module.style ?? 1);
    const isSlide = style === 10;
    const title = module.title ?? {};
    const titleFormat = formatTitle(title);
    const endTime = list[0] ? dateStringToTimestamp(list[0].seckkillData?.seckillEndTime) : 0;

    const contentStyle: CSSProperties = {
        backgroundColor: module.backgroundColor || undefined,
        marginLeft: module.boxPadding ?? 10,
        marginRight: module.boxPadding ?? 10,
        marginBottom: module.boxPaddingBottom ?? 5,
        marginTop: module.boxPaddingTop ?? 5,
        borderRadius: module.boxRadius ?? 0,
        padding: module.innerPadding ?? 0
    };
    const rootStyle: CSSProperties = {
        marginLeft: module.boxPadding ?? 10,
        marginRight: module.boxPadding ?? 10,
        paddingTop: module.boxPaddingTop ?? 5,
        paddingBottom: module.boxPaddingBottom ?? 5,
        backgroundColor: module.backgroundColor || undefined
    };

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad module-promotion-seckill2 ad-style__" +
                module.style +
                " ad-buyBtnStyle__" +
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
            style={rootStyle}
        >
            <div className="module-ad-content" style={contentStyle}>
                {title.showTitle ? (
                    <div className={"image-ad-title ad-title_style__" + (title.titleStyle ?? 1)} style={cssText(titleFormat.titleBackground, titleFormat.titleRadius)}>
                        <div className="image-ad-title-l">
                            <div className="image-ad-title-h" style={{ color: title.titleColor, fontWeight: title.titleWeight }}>
                                {title.titleText}
                            </div>
                            <div className="image-ad-title-d" style={{ color: title.descColor }}>
                                {title.descText}
                            </div>
                        </div>
                        <div className="image-ad-title-r">
                            <H5Countdown countdownType="specifyTime" specifyTime={endTime}>
                                {({ days, hours, minutes, seconds }) => (
                                    <div className="countdown-box">
                                        <span className="countdown-text">距结束</span>
                                        {days && Number(days) > 0 ? (
                                            <>
                                                <span className="countdown-item">{days}</span>
                                                <span className="countdown-separator"> 天 </span>
                                            </>
                                        ) : null}
                                        <span className="countdown-item">{hours}</span>
                                        <span className="countdown-separator">:</span>
                                        <span className="countdown-item">{minutes}</span>
                                        <span className="countdown-separator">:</span>
                                        <span className="countdown-item">{seconds}</span>
                                        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true" style={{ color: "#1a1a1a", marginLeft: 4, verticalAlign: "middle" }}>
                                            <polyline points="9 4.5 16.5 12 9 19.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
                                        </svg>
                                    </div>
                                )}
                            </H5Countdown>
                        </div>
                    </div>
                ) : null}
                <div className="goods-ad-warp">
                    <div
                        className="goods-ad-con"
                        style={isSlide ? { display: "flex", overflow: "hidden", overflowX: "auto", flexWrap: "nowrap" } : undefined}
                    >
                        {list.map((item: any, idx: number) => {
                            const orgPrice = Number(item.orgProductPrice ?? item.marketPrice ?? item.productPrice);
                            return (
                                <div className="goods-ad-item" key={item.id ?? item.productId ?? idx} style={isSlide ? { width: `${100 / 3}%`, flexShrink: 0, flexGrow: 0 } : undefined}>
                                    <div
                                        className="item-content"
                                        style={isSlide ? { paddingLeft: module.goodsPadding ?? 5 } : { padding: module.goodsPadding ?? 5 }}
                                    >
                                        <div className="item-con">
                                            <div className="item-photo">
                                                <div className="item-photo-bg" style={{ backgroundColor: module.imageBackgroundColor }} />
                                                <img src={productImage(item)} alt="" />
                                            </div>
                                            <div className="item-info">
                                                <div className="item-name">
                                                    {module.showName ? <a className="item-name-a">{item.productName}</a> : null}
                                                </div>
                                                <div className="tips">{`优惠${getDiscount(Number(item.productPrice), orgPrice)}%`}</div>
                                                {module.showPrice ? (
                                                    <div className="item-action">
                                                        <div className="item-price" style={isSlide ? { display: "flex", alignItems: "baseline" } : undefined}>
                                                            <span
                                                                className="price_format"
                                                                style={
                                                                    isSlide
                                                                        ? { fontSize: 16, fontWeight: 400, lineHeight: "24px", color: "#1a1a1a", display: "block", height: 24, marginRight: 8 }
                                                                        : undefined
                                                                }
                                                            >
                                                                {priceFormat(Number(item.productPrice))}
                                                            </span>
                                                            {isSlide ? <span className="price-through">{priceFormat(orgPrice)}</span> : null}
                                                        </div>
                                                    </div>
                                                ) : null}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>
            </div>
        </div>
    );
}
