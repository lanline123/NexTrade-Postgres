/**
 * 限时秒杀 预览（DOM 类名逐字对齐 uniapp-react 渲染端 seckill/index.tsx）
 *
 * 商品在渲染端由 getHomeSeckill 接口拉取，编辑器预览用演示商品兜底；
 * 演示字段与接口/渲染端使用一致：productId、productName、seckillPrice、
 * picUrl/picThumb、seckkillData.seckillEndTime（渲染端字段名原文，含拼写）。
 * 倒计时为渲染端 TigCountdown 的等价实现（解析结束时间、每秒重算）。
 */
import { useEffect, useMemo, useState } from "react";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { seckillDefaults } from "./index";
import "./Preview.css";

const DEMO_PICS = [
    "https://oss.tigshop.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071axvkfUDkQNFbIzF1iY.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071lOa0ZBx2mYlvq1lgjr.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071xUnObq2LokKPFecwXq.jpg"
];

/** "YYYY-MM-DD HH:mm:ss" → 时间戳（与渲染端 TigCountdown 解析一致） */
function parseEndTime(endTime: string): number {
    const parts = String(endTime || "").split(" ");
    if (parts.length < 2) return 0;
    const [y, m, d] = parts[0].split("-").map(Number);
    const [hh, mm, ss] = parts[1].split(":").map(Number);
    return new Date(y, (m || 1) - 1, d || 1, hh || 0, mm || 0, ss || 0).getTime();
}

function formatDemoEndTime(offsetMs: number): string {
    const date = new Date(Date.now() + offsetMs);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

/** 渲染端 tig-countdown 等价实现：uni-countdown__number / uni-countdown__splitor 类名一致 */
function TigCountdown({ endTime }: { endTime: string }) {
    const [timeData, setTimeData] = useState({ days: 0, hours: 0, minutes: 0, seconds: 0 });
    useEffect(() => {
        const calc = () => {
            const diff = parseEndTime(endTime) - Date.now();
            if (diff > 0) {
                setTimeData({
                    days: Math.floor(diff / 86400000),
                    hours: Math.floor(diff / 3600000) % 24,
                    minutes: Math.floor(diff / 60000) % 60,
                    seconds: Math.floor(diff / 1000) % 60
                });
            } else {
                setTimeData({ days: 0, hours: 0, minutes: 0, seconds: 0 });
            }
        };
        calc();
        const timer = window.setInterval(calc, 1000);
        return () => window.clearInterval(timer);
    }, [endTime]);
    const fmt = (n: number) => String(n).padStart(2, "0");
    return (
        <div className="tig-countdown" style={{ color: "#fff", fontSize: 12 }}>
            {timeData.days > 0 && (
                <>
                    <span className="uni-countdown__number">{fmt(timeData.days)}</span>
                    <span className="uni-countdown__splitor">:</span>
                </>
            )}
            <span className="uni-countdown__number">{fmt(timeData.hours)}</span>
            <span className="uni-countdown__splitor">:</span>
            <span className="uni-countdown__number">{fmt(timeData.minutes)}</span>
            <span className="uni-countdown__splitor">:</span>
            <span className="uni-countdown__number">{fmt(timeData.seconds)}</span>
        </div>
    );
}

export default function SeckillPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, seckillDefaults);
        return merged;
    }, [data]);

    // 演示商品（仅预览兜底，结束时间相对当前时间生成，保证倒计时有效）
    const demoList = useMemo(
        () =>
            DEMO_PICS.map((pic, i) => ({
                productId: i + 1,
                productName: "演示秒杀商品（编辑模块后由接口加载真实秒杀商品）",
                seckillPrice: 99 + i * 100,
                picUrl: pic,
                picThumb: pic,
                seckkillData: { seckillEndTime: formatDemoEndTime((i + 2) * 3600000) }
            })),
        []
    );

    const frameFormat = formatFrame(module.frame ?? {});
    const allFormat = {
        backgroundColor: module.backgroundColor ? `background-color:${module.backgroundColor};` : "",
        boxPadding: `margin-left:${module.boxPadding ?? 0}px;margin-right:${module.boxPadding ?? 0}px;`,
        boxPaddingBottom: `margin-bottom:${module.boxPaddingBottom ?? 0}px;`,
        boxPaddingTop: `margin-top:${module.boxPaddingTop ?? 0}px;`,
        boxRadius: `border-radius:${module.boxRadius ?? 0}px;`,
        innerPadding: `padding:${module.innerPadding ?? 0}px;`,
        goodsPadding: `padding:${module.goodsPadding ?? 0}px;`
    };
    const tagClassName = (() => {
        switch (Number(module.style)) {
            case 1:
                return "big";
            case 2:
                return "small flex-end";
            case 5:
                return "list flex-end";
            case 6:
                return "small flex-end";
            default:
                return "";
        }
    })();

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad ad-style__" +
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
                {module.title?.showTitle === 1 || module.title?.showTitle === "1" ? <CommonTitle data={module.title} /> : null}
                <div className="goods-ad-warp">
                    <div className="goods-ad-con">
                        {demoList.map((item) => (
                            <div key={item.productId} className="goods-ad-item">
                                <div className="item-content" style={cssText(allFormat.goodsPadding)}>
                                    <div className="item-con">
                                        <div className="item-photo">
                                            <a className="item-image-a">
                                                <img src={productImage(item)} alt="" />
                                            </a>
                                            <div className={"cap-seckill-goods__tag " + tagClassName}>
                                                {Number(module.style) === 1 ? <span className="cap-seckill-goods__tag-title">秒杀</span> : null}
                                                <TigCountdown endTime={item.seckkillData.seckillEndTime} />
                                            </div>
                                        </div>
                                        <div className="item-info">
                                            {module.showName ? (
                                                <div className="item-name">
                                                    <a className="item-name-a">{item.productName}</a>
                                                </div>
                                            ) : null}
                                            {module.showPrice ? (
                                                <div className="item-action">
                                                    <div className="item-price">
                                                        <b className="price_format">{priceFormat(item.seckillPrice)}</b>
                                                    </div>
                                                    <div className="item-buy">
                                                        <a className="buy-btn">
                                                            {Number(module.buyBtnStyle) === 5 || Number(module.buyBtnStyle) === 6 ? (
                                                                <span>购买</span>
                                                            ) : Number(module.buyBtnStyle) === 7 || Number(module.buyBtnStyle) === 8 ? (
                                                                <span>马上抢</span>
                                                            ) : (
                                                                <i className="iconfont-h5" />
                                                            )}
                                                        </a>
                                                    </div>
                                                </div>
                                            ) : (
                                                <div className="blank" style={{ height: 10 }} />
                                            )}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}
