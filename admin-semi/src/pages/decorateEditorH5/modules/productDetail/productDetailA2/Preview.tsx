/**
 * 商品详情 A2（门店风）预览（移植自 ref-editor mobileProductDetail/productDetailA2/Preview.vue + preview.scss）
 * 原版预览合并口径：mergeDefaultModule(module, {...productDetailBaseDefaults, imageMargin: 0})。
 * iconfont 图标（收藏/更多）以同尺寸内联 SVG 替代；分享按钮沿用原版内联 SVG 路径。
 */
import { cloneDeep, mergeDefaultModule } from "../../../contract/sharedDefaults";
import { isMerchant } from "../../../contract/version";
import { useI18n } from "../../../../../i18n";
import ShopInfoPreview from "../shared/ShopInfoPreview";
import { pdImageRatioMap, PD_DEMO_IMAGE, PD_DEMO_DESC_IMAGES, productActionIcons, productDetailBaseDefaults } from "../shared/defaults";
import "./Preview.css";

const titleSizeMap: Record<string, string> = { large: "18px", medium: "15.5px", small: "14px" };
const imageRadiusMap: Record<string, string> = { square: "0px", round: "8px", large: "16px" };
const MORE_ICON = "https://overseas.vortmall.cn/api/static/mini/images/common/more.png";

const Chevron = ({ size = 8 }: { size?: number }) => (
    <svg viewBox="0 0 16 16" width={size} height={size} fill="none">
        <path d="M6 3l5 5-5 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);

export default function Preview({ data }: { data: Record<string, any> }) {
    const { t } = useI18n();
    const module = JSON.parse(JSON.stringify(data ?? {}));
    mergeDefaultModule(module, { ...cloneDeep(productDetailBaseDefaults), imageMargin: 0 });
    const pageBackground = module.backgroundColor && module.backgroundColor !== "transparent" ? module.backgroundColor : "#f5f5f5";
    const cssVars = {
        "--dec-pd2-page-bg": pageBackground,
        "--dec-pd2-content-bg": module.contentBackgroundColor || "#ffffff",
        "--dec-pd2-price-color": module.priceColor || "#fa3534",
        "--dec-pd2-market-price-color": module.marketPriceColor || "#999",
        "--dec-pd2-image-ratio": pdImageRatioMap[module.imageRatio || "1:1"] || "1 / 1",
        "--dec-pd2-image-margin": `${Math.max(0, Number(module.imageMargin ?? 0))}px`,
        "--dec-pd2-image-fit": module.imageMode === 2 ? "cover" : "contain",
        "--dec-pd2-image-radius": imageRadiusMap[module.imageRadiusStyle || "square"] || "0px",
        "--dec-pd2-title-size": titleSizeMap[module.titleSize || "medium"] || "15.5px",
        "--dec-pd2-title-color": module.titleColor || "#2a2a2a",
        "--dec-pd2-title-lines": String(module.titleMaxLines || 2),
        "--dec-pd2-button-radius": `${module.buttonRadius ?? 8}px`
    } as React.CSSProperties;
    const showAnyTool = module.showHome !== false || module.showService !== false || module.showCart !== false;
    return (
        <div className="module-ad-con dec-pd2" style={cssVars}>
            <div className="dec-pd2__hero">
                <div className="dec-pd2__nav-bg" />
                <div className="dec-pd2__navbar">
                    <span className="dec-pd2__nav-btn dec-pd2__nav-btn--plain">
                        <svg viewBox="0 0 16 16" width="17" height="17" fill="none" style={{ transform: "rotate(180deg)" }}>
                            <path d="M6 3l5 5-5 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>
                    </span>
                    <div className="dec-pd2__nav-right">
                        <span className="dec-pd2__nav-btn">
                            <svg className="dec-pd2__nav-share" viewBox="0 0 1024 1024" fill="currentColor" aria-hidden="true">
                                <path
                                    d="M508-63Q389-63 285-3Q207 42 153 112Q99 182 76 269Q53 356 64.5 443.5Q76 531 120.5 609Q165 687 235.5 741Q306 795 393 818Q502 847 610 822Q618 820 622.5 813Q627 806 625 798.5Q623 791 616 786.5Q609 782 602 784Q502 807 403 781Q324 759 259.5 710Q195 661 154 590Q113 519 102.5 438.5Q92 358 113.5 279Q135 200 184 136Q233 72 304 31Q375-10 455.5-21Q536-32 615-10.5Q694 11 758 60Q822 109 863 180Q904 251 914.5 331.5Q925 412 904 491Q902 499 906 506Q910 513 918 515Q926 517 933 513Q940 509 942 501Q965 414 953.5 326.5Q942 239 897 161Q852 83 782 29Q712-25 625-48Q567-63 508-63ZM846 705H730Q722 705 716 710.5Q710 716 710 724Q710 732 716 738Q722 744 730 744H846Q854 744 859.5 738Q865 732 865 724Q865 716 859.5 710.5Q854 705 846 705ZM846 589Q838 589 832 594.5Q826 600 826 609V724Q826 732 832 738Q838 744 846 744Q854 744 859.5 738Q865 732 865 724V609Q865 600 859.5 594.5Q854 589 846 589ZM509 366Q501 366 495 371.5Q489 377 489 385Q489 393 495 399L771 674Q776 680 784 680Q792 680 798 674.5Q804 669 804 661Q804 653 798 647L522 371Q517 366 509 366Z"
                                    transform="translate(0 896) scale(1 -1)"
                                />
                            </svg>
                        </span>
                        <span className="dec-pd2__nav-btn">
                            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.7">
                                <path d="M12 21l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.18L12 21z" />
                            </svg>
                        </span>
                        <span className="dec-pd2__nav-btn">
                            <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
                                <circle cx="5" cy="12" r="1.8" />
                                <circle cx="12" cy="12" r="1.8" />
                                <circle cx="19" cy="12" r="1.8" />
                            </svg>
                        </span>
                    </div>
                </div>
                <div className="dec-pd2__image">
                    <img src={PD_DEMO_IMAGE} alt={t("商品图")} />
                </div>
                <span className="dec-pd2__indicator">
                    <b>1</b>/6
                </span>
            </div>

            <div className="dec-pd2__body">
                <div className="dec-pd2__info-card">
                    <div className="dec-pd2__price">
                        <span className="dec-pd2__price-symbol">¥</span>
                        <span className="dec-pd2__price-value">489</span>
                        <span className="dec-pd2__price-decimal">.00</span>
                        <span className="dec-pd2__market-price">¥599.00</span>
                    </div>
                    <div className="dec-pd2__coupon-row">
                        <span className="dec-pd2__coupon-tag">
                            <span className="dec-pd2__coupon-type">
                                {t("券")}
                                <i className="dec-pd2__coupon-line" />
                            </span>
                            <span className="dec-pd2__coupon-content">{t("满200.00减3...")}</span>
                        </span>
                        <span className="dec-pd2__coupon-tag">
                            <span className="dec-pd2__coupon-type">
                                {t("券")}
                                <i className="dec-pd2__coupon-line" />
                            </span>
                            <span className="dec-pd2__coupon-content">{t("满100.00减2...")}</span>
                        </span>
                        <span className="dec-pd2__coupon-more">
                            {t("领券")}
                            <Chevron />
                        </span>
                    </div>
                    <div className="dec-pd2__title">
                        <span className="dec-pd2__title-text">{t("[商场同款]蹀愫女鞋春秋透气玛丽珍晚晚风方头平底女单鞋")}</span>
                    </div>
                </div>

                <div className="dec-pd2__attr-card">
                    <div className="dec-pd2__attr-row">
                        <span className="dec-pd2__attr-label">{t("规格")}</span>
                        <span className="dec-pd2__attr-value">{t("1件")}</span>
                        <span className="dec-pd2__attr-arrow">
                            <img className="dec-pd2__more" src={MORE_ICON} alt="" />
                        </span>
                    </div>
                    <div className="dec-pd2__attr-row">
                        <span className="dec-pd2__attr-label">{t("销量")}</span>
                        <span className="dec-pd2__attr-value">{t("1021件")}</span>
                    </div>
                    <div className="dec-pd2__attr-row">
                        <span className="dec-pd2__attr-label">{t("参数")}</span>
                        <span className="dec-pd2__attr-value dec-pd2__attr-value--ellipsis">{t("皮革工艺/鞋头款式/闭合方式/风格/流行...")}</span>
                        <span className="dec-pd2__attr-arrow">
                            <img className="dec-pd2__more" src={MORE_ICON} alt="" />
                        </span>
                    </div>
                </div>

                {isMerchant() && module.showShop !== false ? <ShopInfoPreview variant="o2o" /> : null}

                <div className="dec-pd2__review-card">
                    <div className="dec-pd2__review-head">
                        <span className="dec-pd2__review-title">
                            {t("评价")} <em>(0)</em>
                        </span>
                        <span className="dec-pd2__review-rate">
                            {t("好评")}
                            <em>0%</em>
                        </span>
                    </div>
                    <div className="dec-pd2__review-empty">{t("暂无评价~")}</div>
                </div>

                <div className="dec-pd2__detail-card">
                    <div className="dec-pd2__detail-tabs">
                        <span className="dec-pd2__detail-tab dec-pd2__detail-tab--active">{t("商品介绍")}</span>
                        <span className="dec-pd2__detail-tab">{t("售后服务")}</span>
                    </div>
                    <div className="dec-pd2__detail-imgs">
                        {PD_DEMO_DESC_IMAGES.slice(0, 2).map((src) => (
                            <img key={src} src={src} alt="" />
                        ))}
                    </div>
                </div>
            </div>

            <div className={`dec-pd2__buy-bar ${module.buyBarStyle === 2 ? "dec-pd2__buy-bar--square" : "dec-pd2__buy-bar--round"}`}>
                {showAnyTool ? (
                    <div className="dec-pd2__buy-tools">
                        {module.showHome !== false ? (
                            <div className="dec-pd2__buy-tool">
                                <img className="dec-pd2__buy-tool-icon" src={productActionIcons.home} alt="" />
                                <span>{t("首页")}</span>
                            </div>
                        ) : null}
                        {module.showService !== false ? (
                            <div className="dec-pd2__buy-tool">
                                <img className="dec-pd2__buy-tool-icon" src={productActionIcons.service} alt="" />
                                <span>{t("客服")}</span>
                            </div>
                        ) : null}
                        {module.showCart !== false ? (
                            <div className="dec-pd2__buy-tool">
                                <img className="dec-pd2__buy-tool-icon" src={productActionIcons.cart} alt="" />
                                <span>{t("购物车")}</span>
                            </div>
                        ) : null}
                    </div>
                ) : null}
                <div className="dec-pd2__buy-actions">
                    <div className="dec-pd2__buy-btn dec-pd2__buy-btn--cart">{t("加入购物车")}</div>
                    <div className="dec-pd2__buy-btn dec-pd2__buy-btn--buy">{t("立即购买")}</div>
                </div>
            </div>
        </div>
    );
}
