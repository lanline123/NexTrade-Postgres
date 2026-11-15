/**
 * 商品详情 A1（标准风）预览（移植自 ref-editor mobileProductDetail/productDetailA1/Preview.vue + preview.scss）
 * 演示内容（标题/价格/服务项/详情图）为原版硬编码；cssVars 口径与原版 computed 一致。
 * 注：原版各区块为固定排布，不消费 moduleSortList（仅默认值含该字段）。
 */
import { cloneDeep, mergeDefaultModule } from "../../../contract/sharedDefaults";
import { isMerchant } from "../../../contract/version";
import { useI18n } from "../../../../../i18n";
import ShopInfoPreview from "../shared/ShopInfoPreview";
import { createProductDetailDefaults, pdImageRatioMap, PD_DEMO_IMAGE, PD_DEMO_DESC_IMAGES, productActionIcons } from "../shared/defaults";
import "./Preview.css";

const defaults = createProductDetailDefaults("A1");

const Chevron = ({ size = 8, rotate = 0, color = "rgb(136, 139, 148)" }: { size?: number; rotate?: number; color?: string }) => (
    <svg viewBox="0 0 16 16" width={size} height={size} fill="none" style={{ color, transform: rotate ? `rotate(${rotate}deg)` : undefined }}>
        <path d="M6 3l5 5-5 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);

const titleSizeMap: Record<string, string> = { large: "19px", medium: "17px", small: "15px" };
const imageRadiusMap: Record<string, string> = { square: "0px", round: "8px", large: "16px" };

export default function Preview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
    const module = JSON.parse(JSON.stringify(data ?? {}));
    mergeDefaultModule(module, cloneDeep(defaults));
    const cssVars = {
        "--dec-pd-page-bg": module.backgroundColor || "transparent",
        "--dec-pd-content-bg": module.contentBackgroundColor || "#ffffff",
        "--dec-pd-accent-color": module.priceColor || "#ff4d4f",
        "--dec-pd-market-price-color": module.marketPriceColor || "#999",
        "--dec-pd-image-ratio": pdImageRatioMap[module.imageRatio || "4:3"] || "4 / 3",
        "--dec-pd-image-margin": `${Math.max(0, Number(module.imageMargin ?? 16))}px`,
        "--dec-pd-image-fit": module.imageMode === 2 ? "cover" : "contain",
        "--dec-pd-image-radius": imageRadiusMap[module.imageRadiusStyle || "square"] || "0px",
        "--dec-pd-title-size": titleSizeMap[module.titleSize || "medium"] || "17px",
        "--dec-pd-title-color": module.titleColor || "#666666",
        "--dec-pd-title-lines": String(module.titleMaxLines || 2),
        "--dec-pd-button-radius": `${module.buttonRadius ?? 25}px`
    } as React.CSSProperties;
    const showAnyTool = module.showHome !== false || module.showService !== false || module.showCart !== false;
    const serviceItems = [t("售全球"), t("商家发货&售后"), t("准时到达"), t("7天无理由退换货"), t("24小时发货")];
    return (
        <div className="module-ad-con dec-product-detail" style={cssVars}>
            <div className="dec-product-detail__navbar">
                <div className="dec-product-detail__navbar-left">
                    <Chevron size={14} rotate={180} />
                    <span>{t("商品详情")}</span>
                </div>
                <div className="dec-product-detail__navbar-right">
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" style={{ color: "#272c34" }}>
                        <path
                            d="M12 21l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.18L12 21z"
                            stroke="currentColor"
                            strokeWidth="1.6"
                        />
                    </svg>
                    <svg viewBox="0 0 24 24" width="20" height="20" fill="#272c34">
                        <circle cx="5" cy="12" r="1.8" />
                        <circle cx="12" cy="12" r="1.8" />
                        <circle cx="19" cy="12" r="1.8" />
                    </svg>
                </div>
            </div>

            <div className="dec-product-detail__hero">
                <div className="dec-product-detail__image-wrap">
                    <div className="dec-product-detail__image">
                        <img src={PD_DEMO_IMAGE} alt={t("商品图")} />
                    </div>
                </div>
                <div className="dec-product-detail__dots" aria-hidden="true">
                    {[1, 2, 3, 4].map((i) => (
                        <span key={i} className={`dec-product-detail__dot${i === 1 ? " dec-product-detail__dot--active" : ""}`} />
                    ))}
                </div>
                <div className="dec-product-detail__title-section">
                    <div className="dec-product-detail__title">{t("竖条纹衬衫女长袖春季翻领设计感落肩袖斑马纹衬衣女文艺时尚新款")}</div>
                    <div className="dec-product-detail__sales">{t("已售31件")}</div>
                </div>
                <div className="dec-product-detail__price-section">
                    <div className="dec-product-detail__price">
                        <span className="dec-product-detail__price-symbol">$</span>
                        <span className="dec-product-detail__price-value">220</span>
                        <span className="dec-product-detail__price-decimal">.40</span>
                    </div>
                    <div className="dec-product-detail__market-price">$288.00</div>
                </div>
                <div className="dec-product-detail__coupon-card">
                    <span className="dec-product-detail__coupon-text">{t("黄金会员满100元减20元")}</span>
                    <span className="dec-product-detail__coupon-more">
                        <span>{t("更多优惠券")}</span>
                        <Chevron />
                    </span>
                </div>
            </div>

            <div className="dec-product-detail__service-card-box">
                <div className="dec-product-detail__service-card">
                    {serviceItems.map((item) => (
                        <div key={item} className="dec-product-detail__service-row">
                            <span className="dec-product-detail__service-text">{item}</span>
                            <Chevron />
                        </div>
                    ))}
                </div>
            </div>

            {isMerchant() && module.showShop !== false ? <ShopInfoPreview variant="vortmall" /> : null}

            <div className="spec-card-box" style={{ paddingBottom: 0 }}>
                <div className="dec-product-detail__spec-card">
                    <span className="dec-product-detail__spec-title">{t("规格参数")}</span>
                    <Chevron rotate={90} />
                </div>
            </div>

            <div className="spec-card-box" style={{ paddingBottom: 0 }}>
                <div className="dec-product-detail__review-card">
                    <div className="dec-product-detail__review-header">
                        <div className="dec-product-detail__review-left">
                            <span className="dec-product-detail__review-title">{t("评价")}</span>
                            <span className="dec-product-detail__review-tag">
                                <img width="8" height="8" src="https://overseas.vortmall.cn/api/static/mini/images/common/safety.png" alt="" />
                                {t("真实用户评价")}
                            </span>
                        </div>
                        <span className="dec-product-detail__review-more">
                            <span>{t("全部")}</span>
                            <Chevron />
                        </span>
                    </div>
                    <div className="dec-product-detail__review-score">
                        <span className="dec-product-detail__review-stars">★★★★★</span>
                        <span className="dec-product-detail__review-grade">5</span>
                        <span className="dec-product-detail__review-count">{t("(2条评价)")}</span>
                    </div>
                </div>
            </div>

            <div className="spec-card-box" style={{ paddingBottom: 0 }}>
                <div className="dec-product-detail__spec-card">
                    <span className="dec-product-detail__spec-title">{t("售后服务")}</span>
                    <Chevron rotate={90} />
                </div>
            </div>

            <div className="spec-card-box" style={{ paddingBottom: 0 }}>
                <div className="dec-product-detail__spec-card">
                    <span className="dec-product-detail__spec-title">{t("商品描述")}</span>
                    <Chevron rotate={-90} />
                </div>
            </div>

            <div className="image-list-box">
                {PD_DEMO_DESC_IMAGES.map((src) => (
                    <div className="image-list-item" key={src}>
                        <img src={src} alt="" />
                    </div>
                ))}
            </div>

            <div className={`dec-product-detail__buy-bar ${module.buyBarStyle === 2 ? "dec-product-detail__buy-bar--classic" : "dec-product-detail__buy-bar--simple"}`}>
                {showAnyTool ? (
                    <div className="dec-product-detail__buy-bar-tools">
                        {module.showHome !== false ? (
                            <div className="dec-product-detail__buy-bar-tool">
                                <span className="dec-product-detail__buy-bar-tool-icon-wrap">
                                    <img className="dec-product-detail__buy-bar-tool-icon" src={productActionIcons.home} alt="" />
                                </span>
                                <span>{t("首页")}</span>
                            </div>
                        ) : null}
                        {module.showService !== false ? (
                            <div className="dec-product-detail__buy-bar-tool">
                                <span className="dec-product-detail__buy-bar-tool-icon-wrap">
                                    <img className="dec-product-detail__buy-bar-tool-icon" src={productActionIcons.service} alt="" />
                                </span>
                                <span>{t("客服")}</span>
                            </div>
                        ) : null}
                        {module.showCart !== false ? (
                            <div className="dec-product-detail__buy-bar-tool dec-product-detail__buy-bar-tool--cart">
                                <span className="dec-product-detail__buy-bar-tool-icon-wrap">
                                    <img className="dec-product-detail__buy-bar-tool-icon" src={productActionIcons.cart} alt="" />
                                    <span className="dec-product-detail__buy-bar-tool-badge">9</span>
                                </span>
                                <span>{t("购物车")}</span>
                            </div>
                        ) : null}
                    </div>
                ) : null}
                <div className={`dec-product-detail__buy-bar-actions${showAnyTool ? "" : " dec-product-detail__buy-bar-actions--full"}`}>
                    <div className="dec-product-detail__buy-bar-group">
                        <div className="dec-product-detail__buy-bar-btn dec-product-detail__buy-bar-btn--cart">{t("加入购物车")}</div>
                        <div className="dec-product-detail__buy-bar-btn dec-product-detail__buy-bar-btn--buy">{t("立即购买")}</div>
                    </div>
                </div>
            </div>
        </div>
    );
}
