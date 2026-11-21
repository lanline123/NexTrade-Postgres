/**
 * 秒杀2 预览（移植自 view/admin editor/modules/mobilePromotion/seckill2/Preview.vue）
 *
 * 无 frame（defaults 契约）：原版根节点样式为 formatFrame(frame ?? {}) 串 + allFormat.backgroundColor，
 * frame 缺失时 formatFrame 产出的 undefinedpx 无效串被浏览器丢弃，根节点实际只有模块
 * backgroundColor；边距/圆角/内边距全部落在内容层（allFormat），根节点不再另叠一层。
 * 标题区为 CommonTitle + #more 槽倒计时：原版 View.vue 中 #more 槽仅 title.showMore=1
 * 时渲染（槽内容替换默认更多链接），故倒计时仅 showMore=1 时出现、showMore=0 时右侧为空
 * （距结束 + 黑底白字时间块，取首条活动结束时间，以 H5Countdown specifyTime 承接）；
 * 标题图 titleBackgroundPic 有图时按原版 View 渲染 30×30（image-ad-title-img）。
 * 商品同 seckill 取数（原版路径 /promotion/seckill/listForDecorate，无参数、裸数组）。
 * 卡片：item-photo-bg 垫图背景色 imageBackgroundColor、优惠 tips 红 chip（getDiscount 逐字复刻，
 * orgProductPrice 缺失时回退 marketPrice/现价→0，见摘要）、style 10 横滑（一行 3 个、item-content 仅左内边距、
 * 价格 16px #1a1a1a + 划线原价）。
 * 无进行中秒杀活动时以随机真实在售商品兜底展示 4 件（共享 demoProductFill，
 * 纯显示层不落盘；四期与拼团兜底同口径补齐——活动类模块无创建填充，个数仅
 * Preview 层定案；标题区倒计时结束时刻用展示用动态值 demoEndTimeString 承接）。
 */
import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { apiGet } from "../../../../../api";
import H5Countdown from "../../../shared/H5Countdown";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { formatTitle, mergeDefaultModule } from "../../../contract/sharedDefaults";
import { resolveH5DigitalBuyBtn } from "../../../../../components/buyStyleObj";
import { usePcGlobalBuyStyle } from "../../../../decoratePc/previews/shared/PcBuyButton";
import { seckill2Defaults } from "./index";
import { usePreviewDemoProducts, demoEndTimeString } from "../../../../../components/demoProductFill";
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

    // 跟随全局解析（读时、不回写）：buyBtnStyle=-1 时取全局样式降级到 0–8，
    // 与前台 uniapp 同口径（components/buyStyleObj）；其余值原样透传。
    const globalBuyStyle = usePcGlobalBuyStyle();
    const buyBtn = resolveH5DigitalBuyBtn(module.buyBtnStyle, globalBuyStyle);
    const buyBtnStyleVal = buyBtn.style ?? module.buyBtnStyle;


    const [list, setList] = useState<any[]>([]);
    useEffect(() => {
        let alive = true;
        apiGet<any>("/promotion/seckill/listForDecorate")
            .then((res: any) => {
                if (alive) setList(Array.isArray(res) ? res : (res?.records || res?.list || []));
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
    // 无进行中活动时随机真实商品兜底（纯显示层，不落盘）
    const demoFallback = usePreviewDemoProducts(list.length === 0, 4);
    const displayList = list.length
        ? list
        : demoFallback.map((p) => ({ ...p, seckkillData: { seckillEndTime: demoEndTimeString() } }));
    const endTime = displayList[0] ? dateStringToTimestamp(displayList[0].seckkillData?.seckillEndTime) : 0;

    const contentStyle: CSSProperties = {
        backgroundColor: module.backgroundColor || undefined,
        marginLeft: module.boxPadding ?? 10,
        marginRight: module.boxPadding ?? 10,
        marginBottom: module.boxPaddingBottom ?? 5,
        marginTop: module.boxPaddingTop ?? 5,
        borderRadius: module.boxRadius ?? 0,
        padding: module.innerPadding ?? 0
    };
    // 原版根节点仅 allFormat.backgroundColor 生效（frame 缺失的 formatFrame 串为无效 CSS）；
    // 边距/圆角/内边距都在内容层，根节点不再叠加，避免整体多缩进一层
    const rootStyle: CSSProperties = {
        backgroundColor: module.backgroundColor || undefined
    };

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad module-promotion-seckill2 ad-style__" +
                module.style +
                " ad-buyBtnStyle__" +
                buyBtnStyleVal +
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
                            {productImage(title.titleBackgroundPic) ? <img className="image-ad-title-img" src={productImage(title.titleBackgroundPic)} alt="" /> : null}
                            <div className="image-ad-title-h" style={{ color: title.titleColor }}>
                                {title.titleText}
                            </div>
                            <div className="image-ad-title-d" style={{ color: title.descColor }}>
                                {title.descText}
                            </div>
                        </div>
                        {/* 原版 #more 槽：仅 title.showMore=1 时渲染（槽内容即倒计时，无默认更多链接回退） */}
                        {Number(title.showMore) === 1 ? (
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
                        ) : null}
                    </div>
                ) : null}
                <div className="goods-ad-warp">
                    <div
                        className="goods-ad-con"
                        style={isSlide ? { display: "flex", overflow: "hidden", overflowX: "auto", flexWrap: "nowrap" } : undefined}
                    >
                        {displayList.map((item: any, idx: number) => {
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
