/**
 * 搜索结果 预览（搜索头 + 排序 tabs（默认/销量/价格，源自 pages/search/result.tsx 的 tabList）+ 新品 pill 与筛选 chips 为运行时固定项、仅演示；
 * 商品列表排版/配色由模块契约字段驱动，演示商品为占位）
 */
import { useMemo } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { mobileSearchResultDefaults } from "./index";
import "./Preview.css";

const DEMO_GOODS = [
    { name: "演示商品一", price: "19.90", sales: 128 },
    { name: "演示商品二", price: "29.90", sales: 86 }
];

export default function MobileSearchResultPreview({ data }: { data: Record<string, any> }) {
    const module = useMemo(() => {
        const merged = JSON.parse(JSON.stringify(data ?? {}));
        mergeDefaultModule(merged, mobileSearchResultDefaults);
        return merged;
    }, [data]);
    const theme = module.themeColor || "#ff4d4f";
    const priceColor = module.priceColor || theme;
    const listStyle = Number(module.listStyle ?? 1);
    return (
        <div className="module-ad-con dec-search-result" style={{ background: module.backgroundColor || "#f5f5f5" }}>
            <div className="dec-search-result__head" style={{ background: module.searchItemBackgroundColor || "#ffffff", paddingLeft: module.searchItemPaddingLeftRight ?? 10, paddingRight: module.searchItemPaddingLeftRight ?? 10 }}>
                <div className="dec-search-result__search-box">
                    <span className="dec-search-result__placeholder">{module.placeholder}</span>
                    <span className="dec-search-result__search-btn" style={{ background: theme }}>搜索</span>
                </div>
                <div className="dec-search-result__tabs">
                    <span className="dec-search-result__tab is-active" style={{ color: theme }}>默认</span>
                    <span className="dec-search-result__tab">销量</span>
                    <span className="dec-search-result__tab">价格 ↕</span>
                    <span className="dec-search-result__pill">新品</span>
                    <span className="dec-search-result__tab dec-search-result__filter">筛选 ▾</span>
                </div>
            </div>
            <div
                className="dec-search-result__goods-wrap"
                style={{
                    margin: module.margin ?? 0,
                    padding: `${module.paddingTopBottom ?? module.padding ?? 10}px ${module.paddingLeftRight ?? module.padding ?? 10}px`,
                    borderRadius: module.moduleRadius ?? 0,
                    background: module.backgroundColor || undefined
                }}
            >
                <div className={`dec-search-result__goods${listStyle === 2 ? " is-single" : ""}`}>
                    {DEMO_GOODS.map((goods) => (
                        <div className="dec-search-result__item" key={goods.name} style={{ background: module.itemBackgroundColor || "#ffffff", borderRadius: module.productImageRadius ?? 4 }}>
                            <div className="dec-search-result__item-img" style={{ borderRadius: module.productImageRadius ?? 4 }} />
                            <div className="dec-search-result__item-info">
                                <div
                                    className="dec-search-result__item-name"
                                    style={{ color: module.productTitleColor || "#333333", WebkitLineClamp: module.productTitleLine ?? 2 } as React.CSSProperties}
                                >
                                    {goods.name}
                                </div>
                                {listStyle === 2 && module.showShopTag ? <span className="dec-search-result__shop-tag" style={{ color: theme, borderColor: theme }}>店铺</span> : null}
                                {listStyle === 1 && module.showCoupon ? <span className="dec-search-result__coupon-tag" style={{ color: theme, borderColor: theme }}>券</span> : null}
                                <div className="dec-search-result__item-row">
                                    <span className="dec-search-result__item-price" style={{ color: priceColor }}>¥{goods.price}</span>
                                    {listStyle === 2 && module.showSales ? <span className="dec-search-result__item-sales">销量 {goods.sales}</span> : null}
                                    {listStyle === 2 && module.showCartButton ? <span className="dec-search-result__cart-btn" style={{ background: theme }}>+</span> : null}
                                    {listStyle === 1 && Number(module.buyBtnStyle ?? 1) !== 0 ? <span className="dec-search-result__cart-btn" style={{ background: theme }}>+</span> : null}
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
