/**
 * 个性商品 预览（真实渲染）
 *
 * 原版两代编辑器（editor/modules/mobilePro/customProduct/Preview.vue 与更早的
 * decorate/src/modules/mobilePro/customProduct/View.vue）均只渲染占位文本
 * 「个性商品」，无真实预览形态可对齐；此处按 uniapp-react 渲染端
 * （pages/customPage/modules/customProduct）的渲染契约做真实预览，外壳
 * （module-ad-con / module-ad-content + commonStyle，boxRadius2）与原版一致：
 * 海报图按 picLocation（left/right）与商品区左右并置（海报宽占比对齐原版
 * 280/750 设计宽，填充方式取 picFillType，海报底部圆角取 picBottomRadius——
 * 该字段原版无任何渲染先例，此处按字段名语义消费于海报图）；
 * 商品卡 = 图/名/价，颜色取 moduleColor、圆角取 productRadius，
 * crossedOutPrice=1 且有 marketPrice 时显示划线价（crossedOutPriceColor）。
 *
 * pruductList 条目三种形态与渲染端口径一致（2026-11-21 复核收口）：
 * 带 productName/name 的快照对象与纯 ID / {id/productId} 对象统一走共享
 * useDecorateProductList（productSelectType=1）取数，取到最新记录则整体
 * 覆盖快照字段（与渲染端、productCarousel 同口径），未取到回退快照；
 * 组装时按 pruductList 原顺序回填、同一商品不重复出卡。划线价门控与
 * 渲染端一致：crossedOutPrice===1 且 marketPrice 数值 >0 才显示
 * （字符串 "0" 为真值的陷阱已堵）。pruductList 为空显示空态提示，
 * 不写死演示商品。
 */
import { useMemo } from "react";
import { cssText, pickPic, imageFormat, productImage, priceFormat } from "../../../shared/format";
import { useDecorateProductList } from "../../../shared/useDecorateProductList";
import { mergeDefaultModule, formatCommonStyle } from "../../../contract/sharedDefaults";
import { customProductDefaults } from "./index";
import "./Preview.css";

export default function CustomProductPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, customProductDefaults);
        return merged;
    }, [data]);

    const commonStyle = formatCommonStyle(module.moduleStyle ?? {}, module.contentStyle ?? {}, module.subTitle ?? {});
    const mc = module.moduleColor ?? {};

    const raw: Array<any> = Array.isArray(module.pruductList) ? module.pruductList : [];
    // 取数集合与渲染端口径一致：全部条目可解析出的正数 ID（纯 ID / {id|productId} / 快照对象）
    const ids = raw
        .map((x) => (x && typeof x === "object" ? Number(x.productId ?? x.id) : Number(x)))
        .filter((x) => Number.isFinite(x) && x > 0);
    const fetched = useDecorateProductList({ productSelectType: ids.length ? 1 : 0, productIds: ids });
    const fetchedById = useMemo(() => {
        const map = new Map<string, Record<string, any>>();
        fetched.forEach((r) => map.set(String(r.productId ?? r.id), r as Record<string, any>));
        return map;
    }, [fetched]);

    // 按 pruductList 原顺序组装：取到最新记录整体覆盖快照（渲染端同口径）；
    // ID 形态未取到则跳过该位，快照对象未取到回退快照本身
    const displayList: Array<Record<string, any>> = [];
    raw.forEach((x) => {
        const id = x && typeof x === "object" ? Number(x.productId ?? x.id) : Number(x);
        const hit = Number.isFinite(id) && id > 0 ? fetchedById.get(String(id)) : undefined;
        if (x && typeof x === "object" && (x.productName || x.name)) {
            displayList.push(hit ? { ...x, ...hit } : x);
            return;
        }
        if (hit) displayList.push(hit);
    });

    const poster = pickPic(module.picUrl);
    const isLeft = module.picLocation === "left";
    const radius = Number(module.productRadius) || 0;

    return (
        <div
            className="module-ad-con"
            style={cssText(commonStyle.moduleStyle.boxPaddingBottom, commonStyle.moduleStyle.boxPaddingTop, commonStyle.moduleStyle.boxPadding, commonStyle.moduleStyle.backgroundSize, commonStyle.moduleStyle.backgroundImage)}
        >
            <div
                className="module-ad-content"
                style={cssText(commonStyle.moduleContentStyle.backgroundImage, commonStyle.moduleContentStyle.backgroundSize, commonStyle.moduleContentStyle.boxRadius2, commonStyle.moduleContentStyle.boxShadow, commonStyle.moduleContentStyle.innerPadding)}
            >
                <div className="cpr-body" style={{ flexDirection: isLeft ? "row" : "row-reverse" }}>
                    {poster ? (
                        <div
                            className="cpr-poster"
                            style={cssText(Number(module.picBottomRadius) === 2 ? `border-bottom-left-radius:${radius}px;border-bottom-right-radius:${radius}px;` : "")}
                        >
                            <img src={imageFormat(poster)} alt="" style={{ objectFit: module.picFillType || "cover" }} />
                        </div>
                    ) : null}
                    {displayList.length ? (
                        <div className="cpr-grid">
                            {displayList.map((item, idx) => (
                                <div
                                    className="cpr-item"
                                    key={`${item.productId ?? item.id ?? 0}_${idx}`}
                                    style={cssText(`background:${mc.productBackgroundColor || "#fff"};`, `border-radius:${radius}px;`)}
                                >
                                    <img className="cpr-img" src={productImage(item)} alt="" />
                                    <div className="cpr-name" style={cssText(`color:${mc.productNameColor || "#333"};`)}>
                                        {item.productName || item.name}
                                    </div>
                                    <div className="cpr-price-row">
                                        <span className="cpr-price" style={cssText(`color:${mc.productPriceColor || "#72b76c"};`)}>
                                            ¥{priceFormat(item.productPrice ?? item.price)}
                                        </span>
                                        {Number(module.crossedOutPrice) === 1 && Number(item.marketPrice) > 0 ? (
                                            <span className="cpr-market" style={cssText(`color:${mc.crossedOutPriceColor || "#999"};`)}>
                                                ¥{priceFormat(item.marketPrice)}
                                            </span>
                                        ) : null}
                                    </div>
                                </div>
                            ))}
                        </div>
                    ) : (
                        <div className="cpr-empty">{raw.length ? "商品加载中…" : "请先选择商品"}</div>
                    )}
                </div>
            </div>
        </div>
    );
}
