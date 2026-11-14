/**
 * 分类中心 V2 预览（左一级栏 + 二级标签 + 商品区；演示数据为占位，真实分类/商品由渲染端接口下发）
 */
import { useMemo } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { cateCenterV2Defaults } from "./index";
import "./Preview.css";

const DEMO_SIDE = ["热门推荐", "食品饮料", "家居日用"];
const DEMO_TABS = ["全部", "零食", "饮料", "粮油"];
const DEMO_GOODS = [
    { name: "演示商品一", price: "19.90" },
    { name: "演示商品二", price: "29.90" }
];

export default function CateCenterV2Preview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, cateCenterV2Defaults);
        return merged;
    }, [data]);
    const theme = module.themeColor || "#65c4aa";
    const align = Number(module.textAlign) === 2 ? "center" : "left";
    return (
        <div className="module-ad-con dec-cate-v2">
            <div className="dec-cate-v2__search">
                <div className="dec-cate-v2__search-box">🔍 {module.searchPlaceholder}</div>
            </div>
            <div className="dec-cate-v2__body">
                <div className="dec-cate-v2__side" style={{ background: module.sideBgColor || "#f7f7f7" }}>
                    {DEMO_SIDE.map((name, i) => (
                        <div
                            key={name}
                            className={`dec-cate-v2__side-item${i === 0 ? " is-active" : ""}`}
                            style={
                                i === 0
                                    ? { color: module.sideActiveTextColor || theme, background: module.sideActiveBgColor || "#ffffff", borderLeft: `3px solid ${module.sideIndicatorColor || theme}` }
                                    : { color: module.sideTextColor || "#333333", borderLeft: "3px solid transparent" }
                            }
                        >
                            {name}
                        </div>
                    ))}
                </div>
                <div className="dec-cate-v2__main" style={{ background: module.goodsListBgColor || "#ffffff" }}>
                    <div className="dec-cate-v2__tabs">
                        {DEMO_TABS.map((name, i) => (
                            <span
                                key={name}
                                className="dec-cate-v2__tab"
                                style={
                                    i === 0
                                        ? Number(module.subTabStyle) === 2
                                            ? { color: module.subTabActiveTextColor || theme, border: `1px solid ${module.subTabActiveBgColor || theme}`, background: "transparent" }
                                            : { color: module.subTabActiveTextColor || "#ffffff", background: module.subTabActiveBgColor || theme }
                                        : { color: module.subTabTextColor || "#333333", background: module.subTabBgColor || "#f5f5f5" }
                                }
                            >
                                {name}
                            </span>
                        ))}
                    </div>
                    <div className={`dec-cate-v2__goods${Number(module.styleType) === 3 ? " is-list" : ""}`}>
                        {DEMO_GOODS.map((goods) => (
                            <div className="dec-cate-v2__goods-item" key={goods.name}>
                                <div className="dec-cate-v2__goods-img" />
                                <div className="dec-cate-v2__goods-info">
                                    {Number(module.showName ?? 1) !== 0 ? (
                                        <div className="dec-cate-v2__goods-name" style={{ color: module.productTitleColor || "#333333", textAlign: align, fontWeight: Number(module.textWeight) === 2 ? 700 : 400 }}>
                                            {goods.name}
                                        </div>
                                    ) : null}
                                    <div className="dec-cate-v2__goods-row" style={{ justifyContent: align === "center" ? "center" : "space-between" }}>
                                        <span className="dec-cate-v2__goods-price" style={{ color: module.priceColor || theme }}>¥{goods.price}</span>
                                        {Number(module.buyBtnStyle ?? 1) !== 0 ? <span className="dec-cate-v2__buy-btn" style={{ background: module.buyBtnColor || theme }}>+</span> : null}
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
