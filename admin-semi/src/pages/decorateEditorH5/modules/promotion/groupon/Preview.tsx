/**
 * 拼团 预览（移植自 view/admin editor/modules/mobilePromotion/groupon/Preview.vue）
 *
 * 根 class 与 seckill 同构（module-seckill-goods_ad + ad-* 系列），根节点无 frame 外层样式（原版如此）；
 * 内容层 frame 背景/圆角后叠模块 allFormat。数据流同原版 getData：size 按 style 取（<5 取 style、5 取 1），
 * 未选活动或接口列表为空时以原版 defaultGrouponItems 演示数据补足（原版 processData
 * 的 `grouponItemList.value < size` 数组/数字比较怪癖使补足仅在列表为空时发生，
 * 条数不足但非空时不补）；已选活动时取既有接口
 * /promotion/groupon/item/list（后台拼团商品管理同源接口，参数 grouponPromotionId/page/size，不改路径）。
 * 标签条仅 style<3 或 5 显示（style 1 显示「拼团」，倒计时在原版已注释不渲染）；
 * 名称前成团人数 chip 由 getMinGroupNum 逐字复刻（groupNum 为 JSON 数组串时取最小 groupNum，否则 0）；
 * 价格 minGroupPrice + 划线原价（仅 style 1/5），海外版价格色 #555（isOverseas 时）。
 */
import { useEffect, useMemo, useState } from "react";
import { apiGet } from "../../../../../api";
import CommonTitle from "../../../shared/CommonTitle";
import { cssText, productImage, priceFormat } from "../../../shared/format";
import { mergeDefaultModule, formatFrame } from "../../../contract/sharedDefaults";
import { isOverseas } from "../../../contract/version";
import { grouponDefaults } from "./index";
import "./Preview.css";

const DEFAULT_GROUPON_ITEMS = [
    {
        grouponPromotionItemId: 0,
        productId: 0,
        groupNum: 2,
        sort: 1,
        productName: "PP折叠盆",
        productSn: "",
        picLarge: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202510/1761024185GjoiVoxyZCFBgz02sj.jpeg?x-oss-process=image/resize,m_pad,h_800,h_800",
        productPrice: "15.80",
        minGroupPrice: "8.8",
        maxGroupPrice: "12.8",
        minGroupNum: 2,
        maxGroupNum: 5,
        minLimitNum: 10,
        maxLimitNum: 12
    },
    {
        grouponPromotionItemId: 0,
        productId: 0,
        groupNum: 2,
        sort: 1,
        productName: "曲奇饼饼",
        productSn: "",
        picLarge: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202510/17605171914YIcsTf34FQjP0qq3r.jpeg?x-oss-process=image/resize,m_pad,h_800,h_800",
        productPrice: "30.00",
        minGroupPrice: "18.8",
        maxGroupPrice: "26.8",
        minGroupNum: 3,
        maxGroupNum: 8,
        minLimitNum: 10,
        maxLimitNum: 12,
        productBrief: ""
    },
    {
        grouponPromotionItemId: 0,
        productId: 0,
        groupNum: 2,
        sort: 1,
        productName: "脚踏车",
        productSn: "",
        picLarge: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202508/1755141476MeZ9q6HfncgkfbysBG.jpeg?x-oss-process=image/resize,m_pad,h_800,h_800",
        productPrice: "30.00",
        minGroupPrice: "15.8",
        maxGroupPrice: "22.8",
        minGroupNum: 2,
        maxGroupNum: 6,
        minLimitNum: 10,
        maxLimitNum: 12,
        productBrief: ""
    },
    {
        grouponPromotionItemId: 0,
        productId: 0,
        groupNum: 2,
        sort: 1,
        productName: "传统阿胶糕1000g东阿原产即食胶原蛋白阿胶片营养品",
        productSn: "",
        picLarge: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/item/demo/1682046308g5LOOn4OKbSHNNn7kT!!pic800x800.jpeg",
        productPrice: "93.00",
        minGroupPrice: "48.00",
        maxGroupPrice: "68.8",
        minGroupNum: 4,
        maxGroupNum: 10,
        minLimitNum: 10,
        maxLimitNum: 12,
        productBrief: ""
    }
];

/** 成团人数（逐字复刻原版 getMinGroupNum：groupNum 为 JSON 数组串时取最小 groupNum） */
function getMinGroupNum(item: any): number | undefined {
    if (item.groupNum && item.groupNum.length) {
        try {
            const groupNum = JSON.parse(item.groupNum);
            return Math.min(...groupNum.map((x: any) => x.groupNum));
        } catch {
            return 0;
        }
    }
    return undefined;
}

const TAG_CLASS: Record<number, string> = { 1: "big", 2: "small flex-end", 5: "list flex-end", 6: "small flex-end" };

function GrouponBuyButton({ style }: { style: number }) {
    const cartPath = "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
    const cartVariantPath = "M896 149.3H725.3l-27.7-55.5c-8.1-16.2-24.6-26.5-42.7-26.5H369.1c-18.1 0-34.6 10.2-42.7 26.5l-27.7 55.5H128c-35.3 0-64 28.7-64 64v618.7c0 35.3 28.7 64 64 64h768c35.3 0 64-28.7 64-64V213.3c0-35.3-28.7-64-64-64z m-64 576H192V320h640v405.3z";
    const plusPath = "M448 128h128v320h320v128H576v320H448V576H128V448h320V128z";
    const plusVariantPath = "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s14.4-32 32-32H480V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";
    const iconSpan = (path: string) => (
        <span className="iconh5-style">
            <svg viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor" style={{ display: "block" }}>
                <path d={path} />
            </svg>
        </span>
    );
    const render = () => {
        if (style === 5 || style === 6) return <span className={style === 6 ? "activeBtnStyle" : undefined}>购买</span>;
        if (style === 7 || style === 8) return <span className={style === 8 ? "activeBtnStyle" : undefined}>马上抢</span>;
        if (style === 1) return iconSpan(cartPath);
        if (style === 2) return iconSpan(cartVariantPath);
        if (style === 3) return iconSpan(plusPath);
        if (style === 4) return iconSpan(plusVariantPath);
        return null;
    };
    return (
        <a className="buy-btn lyecs-buyBtn" style={{ color: "#555" }}>
            {render()}
        </a>
    );
}

export default function GrouponPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, grouponDefaults);
        return merged;
    }, [data]);

    const style = Number(module.style ?? 1);
    const size = style < 5 ? style : style === 5 ? 1 : 1;
    const activityId = module.grouponActivityItem?.grouponPromotionId;
    const [items, setItems] = useState<any[]>([]);

    useEffect(() => {
        let alive = true;
        // 与原版 processData 一致：仅列表为空时才补演示数据（原版 `list.value < size`
        // 对非空数组恒为 false），条数不足但非空时原样展示，不混入演示商品
        const pad = (list: any[]) => {
            if (list.length > 0) return list;
            const next = [...list];
            let index = 0;
            for (let i = next.length; i < size; i++) {
                next.push(DEFAULT_GROUPON_ITEMS[index]);
                index++;
            }
            return next;
        };
        if (!activityId) {
            setItems(pad([]));
            return;
        }
        apiGet<any>("/promotion/groupon/item/list", { grouponPromotionId: activityId, page: 1, size })
            .then((res: any) => {
                if (alive) setItems(pad(res?.records || res?.list || []));
            })
            .catch(() => {
                if (alive) setItems(pad([]));
            });
        return () => {
            alive = false;
        };
    }, [activityId, size]);

    const frameFormat = formatFrame(module.frame ?? {});
    const allFormat = {
        backgroundColor: module.backgroundColor ? `background-color: ${module.backgroundColor};` : "",
        boxPadding: `margin-left: ${module.boxPadding ?? 10}px;margin-right: ${module.boxPadding ?? 10}px;`,
        boxPaddingBottom: `margin-bottom: ${module.boxPaddingBottom ?? 5}px;`,
        boxPaddingTop: `margin-top: ${module.boxPaddingTop ?? 5}px;`,
        boxRadius: `border-radius: ${module.boxRadius ?? 0}px;`,
        innerPadding: `padding: ${module.innerPadding ?? 0}px;`,
        goodsPadding: `padding: ${module.goodsPadding ?? 5}px;`
    };
    const overseas = isOverseas();
    const priceColor = overseas ? { color: "#555" } : undefined;

    return (
        <div
            className={
                "module-ad-con module-goods_ad module-seckill-goods_ad module-promotion-groupon ad-style__" +
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
                        {items.map((item: any, idx: number) => (
                            <div className="goods-ad-item" key={item.grouponPromotionId ?? item.grouponPromotionItemId ?? idx}>
                                <div className="item-content" style={cssText(allFormat.goodsPadding)}>
                                    <div className="item-con">
                                        <div className="item-photo">
                                            <a className="item-image-a">
                                                <img src={productImage(item)} alt="" />
                                            </a>
                                            {style < 3 || style === 5 ? (
                                                <div className={`cap-seckill-goods__tag ${TAG_CLASS[style] ?? ""}`}>
                                                    {style === 1 ? <span className="cap-seckill-goods__tag-title">拼团</span> : null}
                                                </div>
                                            ) : null}
                                        </div>
                                        <div className="item-info groupon">
                                            <div className="item-name-a">
                                                {module.showName ? (
                                                    <div className="groupon-product-name">
                                                        <span className="group-num">
                                                            <span className="group-num-text">{getMinGroupNum(item) || 0}人团</span>
                                                        </span>{" "}
                                                        {item.productName}
                                                    </div>
                                                ) : null}
                                            </div>
                                            {(style === 1 || style === 5) && item.productBrief ? (
                                                <div className="item-name">
                                                    <div className="item-name-a product-desc">{item.productBrief}</div>
                                                </div>
                                            ) : null}
                                            {module.showPrice ? (
                                                <div className="item-action">
                                                    <div className="item-price">
                                                        <b className="price_format" style={priceColor}>
                                                            {priceFormat(Number(item.minGroupPrice))}
                                                        </b>
                                                        {style === 1 || style === 5 ? (
                                                            <b className="price_format source_price" style={priceColor}>
                                                                {priceFormat(Number(item.productPrice))}
                                                            </b>
                                                        ) : null}
                                                    </div>
                                                    <div className="item-buy">
                                                        <GrouponBuyButton style={Number(module.buyBtnStyle)} />
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
