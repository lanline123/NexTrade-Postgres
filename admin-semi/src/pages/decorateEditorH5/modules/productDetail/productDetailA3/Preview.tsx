/**
 * 商品详情 A3（跨境风）预览（移植自 ref-editor mobileProductDetail/productDetailA3/Preview.vue + preview.scss）
 * - 商城头部：原版 MobilePageHeaderPreview overseas 变体（菜单 + Tigshop logo + 文档/购物车图标），仅海外版且 showHeader!==false 时渲染，底色取 contentBackgroundColor
 * - iconfont 图标（返回/收藏/更多/箭头）以同尺寸内联 SVG 替代
 */
import { cloneDeep, mergeDefaultModule } from "../../../contract/sharedDefaults";
import { isMerchant, isOverseas } from "../../../contract/version";
import { useI18n } from "../../../../../i18n";
import ShopInfoPreview from "../shared/ShopInfoPreview";
import { pdImageRatioMap, PD_DEMO_IMAGE, PD_DEMO_DESC_IMAGES, productDetailBaseDefaults } from "../shared/defaults";
import "./Preview.css";

const titleSizeMap: Record<string, string> = { large: "16px", medium: "15px", small: "14px" };
const imageRadiusMap: Record<string, string> = { square: "0px", round: "10px", large: "18px" };

const Arrow = ({ size = 12 }: { size?: number }) => (
    <svg viewBox="0 0 16 16" width={size} height={size} fill="none">
        <path d="M6 3l5 5-5 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);
const BackIcon = () => (
    <svg viewBox="0 0 16 16" width="16" height="16" fill="none" style={{ transform: "rotate(180deg)" }}>
        <path d="M6 3l5 5-5 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);
const HeartIcon = ({ size = 18 }: { size?: number }) => (
    <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="1.7">
        <path d="M12 21l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.18L12 21z" />
    </svg>
);
const DotsIcon = ({ size = 18 }: { size?: number }) => (
    <svg viewBox="0 0 24 24" width={size} height={size} fill="currentColor">
        <circle cx="5" cy="12" r="1.8" />
        <circle cx="12" cy="12" r="1.8" />
        <circle cx="19" cy="12" r="1.8" />
    </svg>
);

/** 商城头部（原版 MobilePageHeaderPreview overseas 变体） */
function OverseasPageHeader({ backgroundColor }: { backgroundColor: string }) {
    return (
        <div className="dec-mobile-page-header dec-mobile-page-header--overseas" style={{ backgroundColor, display: "flex", alignItems: "center", justifyContent: "space-between", height: 50, padding: "0 12px", color: "#1a1a1a", boxSizing: "border-box" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                    <path d="M4 7h16M4 12h16M4 17h16" />
                </svg>
                <img className="dec-mobile-page-header__logo" src="https://oss.tigshop.com/img/gallery/202506/1749453162yrntnPb4FvQcg9Wjco.png" alt="Tigshop" style={{ height: 22, display: "block" }} />
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
                <svg viewBox="0 0 24 24" width="19" height="19" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M6 2h9l5 5v15H6z" />
                    <path d="M14 2v6h6" />
                </svg>
                <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <circle cx="9" cy="20" r="1.6" />
                    <circle cx="17" cy="20" r="1.6" />
                    <path d="M3 3h2l2.4 12.4a1.5 1.5 0 0 0 1.5 1.1h7.9a1.5 1.5 0 0 0 1.5-1.2L20 8H6" />
                </svg>
            </div>
        </div>
    );
}

export default function Preview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
    const module = JSON.parse(JSON.stringify(data ?? {}));
    mergeDefaultModule(module, cloneDeep(productDetailBaseDefaults));
    const cssVars = {
        "--dec-pd3-page-bg": module.backgroundColor || "#f5f5f5",
        "--dec-pd3-content-bg": module.contentBackgroundColor || "#ffffff",
        "--dec-pd3-price-color": module.priceColor || "#1a1a1a",
        "--dec-pd3-market-price-color": module.marketPriceColor || "#999",
        "--dec-pd3-image-ratio": pdImageRatioMap[module.imageRatio || "1:1"] || "1 / 1",
        "--dec-pd3-image-margin": `${Math.max(0, Number(module.imageMargin ?? 16))}px`,
        "--dec-pd3-image-fit": module.imageMode === 2 ? "cover" : "contain",
        "--dec-pd3-image-radius": imageRadiusMap[module.imageRadiusStyle || "square"] || "0px",
        "--dec-pd3-title-size": titleSizeMap[module.titleSize || "small"] || "14px",
        "--dec-pd3-title-color": module.titleColor || "#1a1a1a",
        "--dec-pd3-title-lines": String(module.titleMaxLines || 2),
        "--dec-pd3-button-radius": `${module.buttonRadius ?? 6}px`
    } as React.CSSProperties;
    return (
        <div className="module-ad-con dec-pd3" style={cssVars}>
            {isOverseas() && module.showHeader !== false ? <OverseasPageHeader backgroundColor={module.contentBackgroundColor || "#ffffff"} /> : null}

            <div className="dec-pd3__nav">
                <span className="dec-pd3__nav-back">
                    <BackIcon />
                </span>
                <div className="dec-pd3__nav-title">{t("商品详情")}</div>
                <span className="dec-pd3__nav-collect">
                    <HeartIcon />
                </span>
                <span className="dec-pd3__nav-more">
                    <DotsIcon />
                </span>
            </div>

            <div className="dec-pd3__name-box">
                <div className="dec-pd3__name">
                    <span className="dec-pd3__name-text">{t("[商场同款]蹀愫女鞋春秋透气玛丽珍晚晚风方头平底女单鞋 舒适百搭")}</span>
                </div>
            </div>
            <div className="dec-pd3__soldout">{t("已售 19 件")}</div>

            <div className="dec-pd3__hero">
                <div className="dec-pd3__image">
                    <img src={PD_DEMO_IMAGE} alt={t("商品图")} />
                </div>
                <div className="dec-pd3__dots">
                    {[0, 1, 2, 3, 4].map((i) => (
                        <i key={i} className={`dec-pd3__dot${i === 0 ? " dec-pd3__dot--active" : ""}`} />
                    ))}
                </div>
            </div>

            <div className="dec-pd3__price-area">
                <div className="dec-pd3__price">
                    <span className="dec-pd3__price-symbol">$</span>
                    <span className="dec-pd3__price-value">489</span>
                    <span className="dec-pd3__price-decimal">.00</span>
                    <span className="dec-pd3__market-price">$599.00</span>
                </div>
            </div>

            <div className="dec-pd3__activity">
                <div className="dec-pd3__activity-inner">
                    <span className="dec-pd3__activity-text">{t("满100减10元")}</span>
                    <span className="dec-pd3__activity-more">{t("更多优惠券")}</span>
                    <span className="dec-pd3__activity-arrow">
                        <Arrow />
                    </span>
                </div>
            </div>

            {isMerchant() && module.showShop !== false ? <ShopInfoPreview variant="overseas" /> : null}

            <div className="dec-pd3__section">
                <span className="dec-pd3__section-title">{t("规格参数")}</span>
                <span className="dec-pd3__section-arrow">
                    <Arrow />
                </span>
            </div>

            <div className="dec-pd3__review">
                <div className="dec-pd3__review-head">
                    <div className="dec-pd3__review-left">
                        <span className="dec-pd3__review-title">{t("评价")}</span>
                        <span className="dec-pd3__review-real">
                            <span className="dec-pd3__review-safe">✓</span>
                            {t("真实用户评价")}
                        </span>
                    </div>
                    <span className="dec-pd3__review-all">
                        {t("全部")}
                        <Arrow />
                    </span>
                </div>
                <div className="dec-pd3__review-score">
                    <span className="dec-pd3__stars">
                        {[1, 2, 3, 4, 5].map((n) => (
                            <span key={n} className="dec-pd3__star">
                                ☆
                            </span>
                        ))}
                    </span>
                    <span className="dec-pd3__score-num">0</span>
                    <span className="dec-pd3__score-total">{t("(0 条评价)")}</span>
                </div>
            </div>

            <div className="dec-pd3__section">
                <span className="dec-pd3__section-title">{t("售后保障")}</span>
                <span className="dec-pd3__section-value">{t("7天无理由退货 · 运费险")}</span>
                <span className="dec-pd3__section-arrow">
                    <Arrow />
                </span>
            </div>

            <div className="dec-pd3__detail">
                <div className="dec-pd3__detail-title">{t("商品详情")}</div>
                <div className="dec-pd3__detail-imgs">
                    {PD_DEMO_DESC_IMAGES.slice(0, 2).map((src) => (
                        <img key={src} src={src} alt="" />
                    ))}
                </div>
            </div>

            <div className={`dec-pd3__footer ${module.buyBarStyle === 2 ? "dec-pd3__footer--square" : "dec-pd3__footer--round"}`}>
                <div className="dec-pd3__footer-btn dec-pd3__footer-btn--cart">{t("加入购物车")}</div>
                <div className="dec-pd3__footer-btn dec-pd3__footer-btn--buy">{t("立即购买")}</div>
            </div>
        </div>
    );
}
