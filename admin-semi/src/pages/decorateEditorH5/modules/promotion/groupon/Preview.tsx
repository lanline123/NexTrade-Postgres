/**
 * 拼团活动 预览（DOM 类名逐字对齐 uniapp-react 渲染端 groupon/index.tsx + index.scss）
 *
 * 活动商品在渲染端由 getHomeGrouponList 接口拉取（并过滤已结束活动），预览用演示
 * 商品兜底；演示字段与接口/渲染端使用一致：productId、productName、productBrief、
 * minGrouponPrice、productPrice、groupNum（JSON 字符串）、endTime、picUrl/picThumb。
 * 成团人数取 groupNum JSON 中的最大值（与渲染端 getMaxGroupNumFromJson 一致）。
 */
import { Fragment, useMemo } from "react";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { grouponDefaults } from "./index";
import "./Preview.css";

const DEMO_PICS = [
    "https://oss.tigshop.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071axvkfUDkQNFbIzF1iY.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071lOa0ZBx2mYlvq1lgjr.jpg",
    "https://oss.tigshop.com/img/gallery/202411/1731919071xUnObq2LokKPFecwXq.jpg"
];

/** 与渲染端 getMaxGroupNumFromJson 一致 */
function getMaxGroupNumFromJson(groupNum: string): number {
    if (!groupNum) return 0;
    try {
        const parsed = JSON.parse(groupNum);
        if (!Array.isArray(parsed)) return 0;
        return parsed.reduce((max: number, item: any) => {
            const current = Number(item?.groupNum);
            return Number.isFinite(current) ? Math.max(max, current) : max;
        }, 0);
    } catch {
        return 0;
    }
}

export default function GrouponPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, grouponDefaults);
        return merged;
    }, [data]);

    // 演示商品（仅预览兜底）
    const demoList = useMemo(
        () =>
            DEMO_PICS.map((pic, i) => ({
                productId: i + 1,
                productName: "演示拼团商品（编辑模块后由接口加载真实拼团商品）",
                productBrief: "拼团立减，邀好友一起拼",
                minGrouponPrice: 79 + i * 100,
                productPrice: 129 + i * 100,
                groupNum: JSON.stringify([{ groupNum: 2 }, { groupNum: 5 }]),
                endTime: "2099-12-31 23:59:59",
                picUrl: pic,
                picThumb: pic,
                maxGroupNum: 5
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
        switch (String(module.style)) {
            case "1":
                return "big";
            case "2":
                return "small flex-end";
            case "5":
                return "list flex-end";
            case "6":
                return "small flex-end";
            default:
                return "";
        }
    })();
    const styleNum = Number(module.style);

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad module-groupon ad-style__" +
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
                        <Fragment key={item.productId}>
                            <div className="goods-ad-item">
                                <div className="item-content" style={cssText(allFormat.goodsPadding)}>
                                    <div className="item-con">
                                        <div className="item-photo">
                                            <a className="item-image-a">
                                                <img src={productImage(item)} alt="" />
                                            </a>
                                            {(styleNum < 3 || styleNum === 5) && (
                                                <div className={"cap-seckill-goods__tag " + tagClassName}>
                                                    {styleNum === 1 ? <span className="cap-seckill-goods__tag-title">拼团</span> : null}
                                                </div>
                                            )}
                                        </div>
                                        <div className="item-info">
                                            <div className="item-name">
                                                {module.showName ? (
                                                    <a className="item-name-a line2">
                                                        <span className="group-num">
                                                            <span className="group-num-text">{item.maxGroupNum || getMaxGroupNumFromJson(item.groupNum) || 0}人团</span>
                                                        </span>
                                                        <span className="item-name-link">{item.productName}</span>
                                                    </a>
                                                ) : null}
                                                {(styleNum === 1 || styleNum === 5) && module.grouponActivityItem?.productBrief ? (
                                                    <div className="item-brief product-desc">
                                                        <span>{item.productBrief}</span>
                                                    </div>
                                                ) : null}
                                            </div>
                                            {module.showPrice ? (
                                                <div className="item-action">
                                                    <div className="item-price">
                                                        <b className="price_format">{priceFormat(item.minGrouponPrice)}</b>
                                                        {styleNum === 1 || styleNum === 5 ? (
                                                            <span style={{ marginLeft: 4, color: "#ccc", textDecoration: "line-through", fontSize: 12 }}>
                                                                {priceFormat(item.productPrice)}
                                                            </span>
                                                        ) : null}
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
                        </Fragment>
                    ))}
                </div>
            </div>
        </div>
    );
}
