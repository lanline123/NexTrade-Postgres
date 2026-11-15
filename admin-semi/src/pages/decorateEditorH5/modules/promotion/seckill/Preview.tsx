/**
 * 秒杀 预览（移植自 view/admin editor/modules/mobilePromotion/seckill/Preview.vue）
 *
 * 根 class 与 Vue 版逐一致（module-seckill-goods_ad + ad-style__/ad-buyBtnStyle__/ad-goods_style__/...），
 * 样式由全局 module.css 提供；外层 frame 外边距、内容层 frame 背景/圆角后叠模块级 allFormat
 * （backgroundColor/boxPadding→margin/boxRadius/innerPadding）。CommonTitle（title.showTitle 门控）。
 * 商品数据：原版 getMobileSeckillList 实时拉取——本实现取后台既有接口
 * /promotion/seckill/listForHomePreview（与 PC 装修秒杀预览同源同方法，size 15），不新增/不改接口路径。
 * 每卡图上 .cap-seckill-goods__tag（style 1 显示「秒杀」+ 倒计时 DD天 HH:mm:ss 白字 12px/700，
 * 类名按 style 映射 big/small flex-end/list flex-end）；购买按钮为旧数字 buyBtnStyle 语义（同 product 模块）。
 */
import { useEffect, useMemo, useState } from "react";
import { apiGet } from "../../../../../api";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { seckillDefaults } from "./index";
import "./Preview.css";

function LegacyBuyButton({ style }: { style: number }) {
    const cartPath = "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
    const cartVariantPath = "M896 149.3H725.3l-27.7-55.5c-8.1-16.2-24.6-26.5-42.7-26.5H369.1c-18.1 0-34.6 10.2-42.7 26.5l-27.7 55.5H128c-35.3 0-64 28.7-64 64v618.7c0 35.3 28.7 64 64 64h768c35.3 0 64-28.7 64-64V213.3c0-35.3-28.7-64-64-64z m-64 576H192V320h640v405.3z";
    const plusPath = "M448 128h128v320h320v128H576v320H448V576H128V448h320V128z";
    const plusVariantPath = "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s32-14.4 32-32h181.3V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";
    const iconSpan = (path: string) => (
        <span className="iconh5-style">
            <svg viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor" style={{ display: "block" }}>
                <path d={path} />
            </svg>
        </span>
    );
    const render = () => {
        if (style === 5 || style === 6) return <span>购买</span>;
        if (style === 7 || style === 8) return <span>马上抢</span>;
        if (style === 1) return iconSpan(cartPath);
        if (style === 2) return iconSpan(cartVariantPath);
        if (style === 3) return iconSpan(plusPath);
        if (style === 4) return iconSpan(plusVariantPath);
        return null;
    };
    return <a className="buy-btn lyecs-buyBtn">{render()}</a>;
}

/** "YYYY-MM-DD HH:mm:ss" → 时间戳（同原版 dateStringToTimestamp） */
function dateStringToTimestamp(dateString?: string): number {
    if (!dateString) return 0;
    const parts = String(dateString).split(" ");
    const [year, month, day] = (parts[0] || "").split("-");
    const [hour, minute, second] = (parts[1] || "0:0:0").split(":");
    return new Date(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second)).getTime();
}

/** el-countdown format="DD[天] HH:mm:ss" 等价格式化 */
function formatCountdown(end: number, now: number): string {
    let diff = Math.max(0, Math.floor((end - now) / 1000));
    const d = Math.floor(diff / 86400);
    diff -= d * 86400;
    const h = Math.floor(diff / 3600);
    diff -= h * 3600;
    const m = Math.floor(diff / 60);
    const s = diff - m * 60;
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${pad(d)}天 ${pad(h)}:${pad(m)}:${pad(s)}`;
}

const TAG_CLASS: Record<number, string> = { 1: "big", 2: "small flex-end", 5: "list flex-end", 6: "small flex-end" };

export default function SeckillPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, seckillDefaults);
        return merged;
    }, [data]);

    const [list, setList] = useState<any[]>([]);
    const [now, setNow] = useState(() => Date.now());
    useEffect(() => {
        let alive = true;
        apiGet<any>("/promotion/seckill/listForHomePreview", { page: 1, size: 15 })
            .then((res: any) => {
                if (alive) setList(res?.records || res?.list || (Array.isArray(res) ? res : []));
            })
            .catch(() => undefined);
        const timer = window.setInterval(() => setNow(Date.now()), 1000);
        return () => {
            alive = false;
            window.clearInterval(timer);
        };
    }, []);

    const frameFormat = formatFrame(module.frame ?? {});
    const allFormat = {
        backgroundColor: module.backgroundColor ? `background-color: ${module.backgroundColor};` : "",
        boxPadding: `margin-left: ${module.boxPadding ?? 10}px;margin-right: ${module.boxPadding ?? 10}px;`,
        boxPaddingBottom: `margin-bottom: ${module.boxPaddingBottom ?? 5}px;`,
        boxPaddingTop: `margin-top: ${module.boxPaddingTop ?? 5}px;`,
        boxRadius: `border-radius: ${module.boxRadius ?? 0}px;`,
        innerPadding: `padding: ${module.innerPadding ?? 0}px;`
    };
    const goodsPadding = cssText(`padding: ${module.goodsPadding ?? 5}px;`);
    const style = Number(module.style ?? 1);

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad ad-style__" +
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
            style={cssText(frameFormat.boxPadding, frameFormat.boxPaddingTop, frameFormat.boxPaddingBottom)}
        >
            <div
                className="module-ad-content"
                style={cssText(
                    frameFormat.backgroundColor,
                    frameFormat.boxRadius,
                    allFormat.backgroundColor,
                    allFormat.boxPadding,
                    allFormat.boxPaddingBottom,
                    allFormat.boxPaddingTop,
                    allFormat.boxRadius,
                    allFormat.innerPadding
                )}
            >
                <CommonTitle data={module.title ?? {}} />
                <div className="goods-ad-warp">
                    <div className="goods-ad-con">
                        {list.map((item: any, idx: number) => (
                            <div className="goods-ad-item" key={item.id ?? item.productId ?? idx}>
                                <div className="item-content" style={goodsPadding}>
                                    <div className="item-con">
                                        <div className="item-photo">
                                            <a className="item-image-a">
                                                <img src={productImage(item)} alt="" />
                                            </a>
                                            <div className={`cap-seckill-goods__tag ${TAG_CLASS[style] ?? ""}`}>
                                                {style === 1 ? <span className="cap-seckill-goods__tag-title">秒杀</span> : null}
                                                <span className="cap-seckill-goods__tag-countdown">
                                                    {formatCountdown(dateStringToTimestamp(item.seckkillData?.seckillEndTime), now)}
                                                </span>
                                            </div>
                                        </div>
                                        <div className="item-info">
                                            <div className="item-name">
                                                {module.showName ? <a className="item-name-a">{item.productName}</a> : null}
                                            </div>
                                            {module.showPrice ? (
                                                <div className="item-action">
                                                    <div className="item-price">
                                                        <b className="price_format">{priceFormat(Number(item.productPrice))}</b>
                                                    </div>
                                                    <div className="item-buy">
                                                        <LegacyBuyButton style={Number(module.buyBtnStyle)} />
                                                    </div>
                                                </div>
                                            ) : null}
                                            <div className="blank" style={{ height: 10 }} />
                                        </div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                    <div className="swiper-pagination-con">
                        <div className="swiper-pagination" />
                    </div>
                </div>
            </div>
        </div>
    );
}
