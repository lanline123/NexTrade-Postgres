/**
 * 搜索结果 预览（移植自 view/admin editor/modules/mobileSearch/mobileSearchResult/Preview.vue + preview.less）
 *
 * - 根节点 module-ad-con（选中蓝框画在这里）；模块外边距只作用商品列表 __content，不包搜索头/筛选。
 * - 头部为原版 MobilePageHeaderPreview 的内联复刻（画布对 pageType=6 不渲染页面头，
 *   搜索页头部收敛在本模块内）：domestic = 返回 + 居中标题「商品搜索」；
 *   overseas = 菜单 + logo + 文档/购物车（角标 21）。门店模式固定国内版头部。
 * - iconfont 图标以内联 SVG 承接（返回/搜索/价格升降三角/筛选/下拉/关注/菜单/文档/购物车），
 *   购物车按钮 1~4 的字形与 basic/product 的 LegacyBuyButton 同源（icon-gouwuche/gouwuche1/jia/jia1）。
 * - 星级门店标底图 tag-bg.png 为原版 admin 本地资源（ref 快照中 assets 缺失、本工程亦无），
 *   以主题色渐变底承接（见小结）。
 * - 无数据接口：原版 Preview 即静态 mock（4 商品 / 2 门店），此处逐字复刻演示数据。
 */
import { useMemo, type CSSProperties } from "react";
import { mergeDefaultModule } from "../../../contract/sharedDefaults";
import { mobileSearchResultDefaults } from "./index";
import "./Preview.css";

/* ---------------- 图标（iconfont 等价内联 SVG） ---------------- */

function ChevronLeft({ size = 14 }: { size?: number }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ display: "block", flexShrink: 0 }}>
            <polyline points="15 4.5 7.5 12 15 19.5" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function ChevronDown({ size = 10 }: { size?: number }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ display: "block", flexShrink: 0 }}>
            <polyline points="4.5 9 12 16.5 19.5 9" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function SearchIcon({ size = 18 }: { size?: number }) {
    return (
        <svg viewBox="0 0 1024 1024" width={size} height={size} fill="currentColor" aria-hidden="true" style={{ display: "block", flexShrink: 0 }}>
            <path d="M448 768A320 320 0 1 0 448 128a320 320 0 0 0 0 640z m297.6 43.3a448 448 0 1 1 66.7-66.7l194 194a48 48 0 0 1-67.9 67.9l-192.8-195.2z" />
        </svg>
    );
}

function Triangle({ size = 8, up = false }: { size?: number; up?: boolean }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ display: "block" }}>
            <path d={up ? "M12 5l8 12H4z" : "M12 19L4 7h16z"} fill="currentColor" />
        </svg>
    );
}

function FilterIcon({ size = 12 }: { size?: number }) {
    return (
        <svg viewBox="0 0 1024 1024" width={size} height={size} fill="currentColor" aria-hidden="true" style={{ display: "block", flexShrink: 0 }}>
            <path d="M128 128h768c17.7 0 26.5 21.4 14 33.9L608 464v320c0 8.8-4.9 17-12.6 21.2l-128 64c-14.9 7.4-35.4-3.6-35.4-21.2V464L114 161.9C101.4 149.4 110.3 128 128 128z" />
        </svg>
    );
}

function HeartIcon({ size = 11 }: { size?: number }) {
    return (
        <svg viewBox="0 0 1024 1024" width={size} height={size} fill="currentColor" aria-hidden="true" style={{ display: "block" }}>
            <path d="M512 896C320 736 96 592 96 352 96 208 192 128 288 128c80 0 160 48 224 128 64-80 144-128 224-128 96 0 192 80 192 224 0 240-224 384-416 544z" />
        </svg>
    );
}

function MenuIcon({ size = 21 }: { size?: number }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ display: "block" }}>
            <path d="M4 7h16M4 12h16M4 17h16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
    );
}

function DocIcon({ size = 19 }: { size?: number }) {
    return (
        <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true" style={{ display: "block" }}>
            <path d="M6 3h8l4 4v14H6z" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
            <path d="M14 3v4h4" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
        </svg>
    );
}

const CART_PATH =
    "M401.3 829.9c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m400.4 0c-35.8 0-64.9 29.1-64.9 64.9s29.1 64.9 64.9 64.9 64.9-29.1 64.9-64.9-29.1-64.9-64.9-64.9z m211.9-622.1c-11-12.7-27.2-19.9-44.1-19.9H238.9l-17.4-96.8C217.4 55.4 185.5 29 149.5 29H63.4C37.8 29 17 49.8 17 75.4s20.8 46.4 46.4 46.4h86.1c3.2 0 5.9 2.3 6.5 5.4l76.7 425.7c8.5 47.6 49.8 82.1 98.2 82.1h438.3c46.4 0 86.9-33 96.2-78.5l54.7-270.4c3.4-18.1-1.3-36.6-13.5-50.3z";
const CART_VARIANT_PATH =
    "M896 149.3H725.3l-27.7-55.5c-8.1-16.2-24.6-26.5-42.7-26.5H369.1c-18.1 0-34.6 10.2-42.7 26.5l-27.7 55.5H128c-35.3 0-64 28.7-64 64v618.7c0 35.3 28.7 64 64 64h768c35.3 0 64-28.7 64-64V213.3c0-35.3-28.7-64-64-64z m-64 576H192V320h640v405.3z";
const PLUS_PATH = "M448 128h128v320h320v128H576v320H448V576H128V448h320V128z";
const PLUS_VARIANT_PATH =
    "M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64z m213.3 480H544v181.3c0 17.6-14.4 32-32 32s-32-14.4-32-32V544H298.7c-17.6 0-32-14.4-32-32s14.4-32 32-32H480V298.7c0-17.6 14.4-32 32-32s32 14.4 32 32V480h181.3c17.6 0 32 14.4 32 32s-14.4 32-32 32z";

function CartGlyph({ size = 21 }: { size?: number }) {
    return (
        <svg viewBox="0 0 1024 1024" width={size} height={size} fill="currentColor" aria-hidden="true" style={{ display: "block" }}>
            <path d={CART_PATH} />
        </svg>
    );
}

function BuyBtnIcon({ style }: { style: number }) {
    const path = style === 1 ? CART_PATH : style === 2 ? CART_VARIANT_PATH : style === 3 ? PLUS_PATH : PLUS_VARIANT_PATH;
    return (
        <svg viewBox="0 0 1024 1024" width="1em" height="1em" fill="currentColor" aria-hidden="true" style={{ display: "block" }}>
            <path d={path} />
        </svg>
    );
}

/* ---------------- 归一化（与原版 Preview.vue 的 toBool/toNum/toRgba 逐一对应） ---------------- */

const toBool = (v: unknown, fallback: boolean): boolean => {
    if (v === undefined || v === null || v === "") return fallback;
    if (typeof v === "boolean") return v;
    if (typeof v === "number") return v !== 0;
    if (typeof v === "string") {
        const s = v.trim().toLowerCase();
        if (s === "true" || s === "1") return true;
        if (s === "false" || s === "0") return false;
    }
    return fallback;
};
const toNum = (v: unknown, fallback: number): number => {
    if (v === undefined || v === null || v === "") return fallback;
    const n = typeof v === "number" ? v : Number(v);
    return Number.isFinite(n) ? n : fallback;
};
function clampNumber(n: number, min: number, max: number) {
    return Math.min(max, Math.max(min, n));
}
function toRgba(color: string | undefined, alpha: number): string {
    const a = clampNumber(alpha, 0, 1);
    const fallback = `rgba(255, 77, 79, ${a})`;
    if (!color) return fallback;
    const c = color.trim();
    if (!c) return fallback;
    if (/^rgba?\(/i.test(c)) {
        const match = c.match(/rgba?\(\s*([0-9.]+)\s*,\s*([0-9.]+)\s*,\s*([0-9.]+)(?:\s*,\s*([0-9.]+)\s*)?\)/i);
        if (!match) return fallback;
        const r = Number(match[1]);
        const g = Number(match[2]);
        const b = Number(match[3]);
        if ([r, g, b].some((v) => Number.isNaN(v))) return fallback;
        return `rgba(${r}, ${g}, ${b}, ${a})`;
    }
    if (c.startsWith("#")) {
        const hex = c.slice(1);
        const normalized = hex.length === 3 ? hex.split("").map((ch) => ch + ch).join("") : hex;
        if (!/^[0-9a-fA-F]{6}$/.test(normalized)) return fallback;
        const r = parseInt(normalized.slice(0, 2), 16);
        const g = parseInt(normalized.slice(2, 4), 16);
        const b = parseInt(normalized.slice(4, 6), 16);
        return `rgba(${r}, ${g}, ${b}, ${a})`;
    }
    return fallback;
}

/* ---------------- 演示数据（原版 Preview.vue mockProducts / mockStores 逐字） ---------------- */

const mockProducts = [
    {
        id: 1,
        title: "小米（MI）Xiaomi Watch S3 银色 国家补贴 澎湃OS 全天血氧监测 智能运动手表 男款女款",
        image: "https://oss.tigshop.com/img/item/demo/1680762243Myvkn91XZndQaLrMTf!!pic400x400.jpeg",
        price: "649",
        quantity: 1
    },
    {
        id: 2,
        title: "羽绒服 冬季加厚保暖 大鹅绒填充 户外防风外套",
        image: "https://oss.tigshop.com/img/item/demo/1680589546tPrOZyKK683sfEOCq7!!pic400x400.jpeg",
        price: "5499",
        quantity: 1
    },
    {
        id: 3,
        title: "女秋冬加绒加厚大喇叭裤黑色西装裤子高腰修身显瘦垂感休闲拖地裤",
        image: "https://oss.tigshop.com/img/item/demo/16807622759lrB7tSYTaDMl21ver!!pic400x400.jpeg",
        price: "3679",
        quantity: 1
    },
    {
        id: 4,
        title: "双面羊绒大衣 秋冬新款 通勤百搭 中长款毛呢外套",
        image: "https://oss.tigshop.com/img/item/demo/1680762215SVhVhwTAK3TCRQdOQC!!pic400x400.jpeg",
        price: "2799",
        quantity: 1
    }
];

const mockStores = [
    {
        id: 1,
        title: "Tigshop 官方旗舰门店（望京店）",
        logo: "https://oss.tigshop.com/img/item/demo/1680762243Myvkn91XZndQaLrMTf!!pic400x400.jpeg",
        score: "4.9",
        minOrder: "20.00",
        deliveryFee: "3.00",
        distance: "860m",
        products: mockProducts
    },
    {
        id: 2,
        title: "Tigshop 品牌直营店（中关村店）",
        logo: "https://oss.tigshop.com/img/item/demo/1680589546tPrOZyKK683sfEOCq7!!pic400x400.jpeg",
        score: "4.8",
        minOrder: "30.00",
        deliveryFee: "5.00",
        distance: "2.30km",
        products: mockProducts
    }
];

const OVERSEAS_FILTER_TABS = ["排序", "品牌", "图案", "厚薄"];
const HEADER_LOGO = "https://oss.tigshop.com/img/gallery/202506/1749453162yrntnPb4FvQcg9Wjco.png";

/** 页面头模拟件（原版 MobilePageHeaderPreview.vue 内联复刻，样式见 Preview.css 的 dec-mobile-page-header*） */
function PageHeaderPreview({ variant, cartCount }: { variant: "domestic" | "overseas"; cartCount?: string }) {
    if (variant === "overseas") {
        return (
            <div className="dec-mobile-page-header dec-mobile-page-header--overseas">
                <div className="dec-mobile-page-header__overseas-left">
                    <MenuIcon />
                    <img className="dec-mobile-page-header__logo" src={HEADER_LOGO} alt="Tigshop" />
                </div>
                <div className="dec-mobile-page-header__overseas-right">
                    <DocIcon />
                    <span className="dec-mobile-page-header__cart">
                        <CartGlyph />
                        {cartCount ? <span className="dec-mobile-page-header__cart-badge">{cartCount}</span> : null}
                    </span>
                </div>
            </div>
        );
    }
    return (
        <div className="dec-mobile-page-header dec-mobile-page-header--domestic">
            <span className="dec-mobile-page-header__back">
                <ChevronLeft size={16} />
            </span>
            <div className="dec-mobile-page-header__title">商品搜索</div>
        </div>
    );
}

export default function MobileSearchResultPreview({ data }: { data: Record<string, any> }) {
    const cfg: Record<string, any> = useMemo(() => {
        const raw: Record<string, any> = JSON.parse(JSON.stringify(data ?? {}));
        // 历史兼容：新字段不存在时先继承旧单一 padding，再 merge 默认值（同原版 moduleConfig 的 fallback 链）
        if (raw.paddingTopBottom === undefined || raw.paddingTopBottom === null || raw.paddingTopBottom === "") {
            if (raw.padding !== undefined && raw.padding !== null && raw.padding !== "") raw.paddingTopBottom = raw.padding;
        }
        if (raw.paddingLeftRight === undefined || raw.paddingLeftRight === null || raw.paddingLeftRight === "") {
            if (raw.padding !== undefined && raw.padding !== null && raw.padding !== "") raw.paddingLeftRight = raw.padding;
        }
        mergeDefaultModule(raw, mobileSearchResultDefaults);
        const d = mobileSearchResultDefaults;
        return {
            ...raw,
            showShopTag: toBool(raw.showShopTag, d.showShopTag),
            showSales: toBool(raw.showSales, d.showSales),
            showCartButton: toBool(raw.showCartButton, d.showCartButton),
            showCoupon: toBool(raw.showCoupon, d.showCoupon),
            layoutStyle: toNum(raw.layoutStyle, d.layoutStyle) as 1 | 2,
            listStyle: toNum(raw.listStyle, d.listStyle) as 1 | 2,
            productTitleLine: toNum(raw.productTitleLine, d.productTitleLine) as 1 | 2,
            buyBtnStyle: toNum(raw.buyBtnStyle, d.buyBtnStyle),
            searchItemPaddingLeftRight: toNum(raw.searchItemPaddingLeftRight, d.searchItemPaddingLeftRight),
            margin: toNum(raw.margin, d.margin),
            padding: toNum(raw.padding, d.padding),
            paddingTopBottom: toNum(raw.paddingTopBottom, d.paddingTopBottom),
            paddingLeftRight: toNum(raw.paddingLeftRight, d.paddingLeftRight),
            moduleRadius: toNum(raw.moduleRadius, d.moduleRadius),
            productImageRadius: toNum(raw.productImageRadius, d.productImageRadius),
            searchResultType: toNum(raw.searchResultType, d.searchResultType) as 1 | 2,
            storeShowRating: toNum(raw.storeShowRating, d.storeShowRating),
            storeShowMinOrder: toNum(raw.storeShowMinOrder, d.storeShowMinOrder),
            storeShowDeliveryFee: toNum(raw.storeShowDeliveryFee, d.storeShowDeliveryFee),
            storeShowDistance: toNum(raw.storeShowDistance, d.storeShowDistance),
            storeShowFollow: toNum(raw.storeShowFollow, d.storeShowFollow),
            storeShowProductName: toNum(raw.storeShowProductName, d.storeShowProductName),
            storeProductNameRow: toNum(raw.storeProductNameRow, d.storeProductNameRow) as 1 | 2,
            storeShowProductPrice: toNum(raw.storeShowProductPrice, d.storeShowProductPrice),
            storeProductGap: toNum(raw.storeProductGap, d.storeProductGap),
            storeGap: toNum(raw.storeGap, d.storeGap),
            storePaddingTopBottom: toNum(raw.storePaddingTopBottom, d.storePaddingTopBottom),
            storePaddingLeftRight: toNum(raw.storePaddingLeftRight, d.storePaddingLeftRight),
            storeCardRadius: toNum(raw.storeCardRadius, d.storeCardRadius),
            storeLogoRadius: toNum(raw.storeLogoRadius, d.storeLogoRadius),
            storeShopTitleFontSize: toNum(raw.storeShopTitleFontSize, d.storeShopTitleFontSize),
            storeShopTitleFontBold: toNum(raw.storeShopTitleFontBold, d.storeShopTitleFontBold)
        };
    }, [data]);

    const buyBtnStyle = Number(cfg.buyBtnStyle ?? 0);
    const isStoreResult = cfg.searchResultType === 2;
    /** 门店聚合搜索固定国内版搜索头，商品模式才读 layoutStyle */
    const headerLayoutStyle = isStoreResult ? 1 : Number(cfg.layoutStyle ?? 1);
    const storeVisible = {
        showRating: cfg.storeShowRating === 1,
        showMinOrder: cfg.storeShowMinOrder === 1,
        showDeliveryFee: cfg.storeShowDeliveryFee === 1,
        showDistance: cfg.storeShowDistance === 1,
        showFollow: cfg.storeShowFollow === 1,
        showProductName: cfg.storeShowProductName === 1,
        showProductPrice: cfg.storeShowProductPrice === 1
    };
    const storeProductGap = Math.max(0, cfg.storeProductGap ?? 8);
    const storeGap = Math.max(0, cfg.storeGap ?? 10);
    const storeCardRadius = Math.max(0, cfg.storeCardRadius ?? 8);
    const storeCardStyle: CSSProperties = {
        borderRadius: `${storeCardRadius}px`,
        overflow: "hidden",
        padding: `${Math.max(0, cfg.storePaddingTopBottom ?? 12)}px ${Math.max(0, cfg.storePaddingLeftRight ?? 12)}px`
    };
    const storeLogoRadius = Math.min(50, Math.max(0, cfg.storeLogoRadius ?? 50));
    const storeProductNameRow = cfg.storeProductNameRow === 2 ? 2 : 1;
    const storeTitleStyle: CSSProperties = {
        color: cfg.storeShopTitleColor || "#1a1a1a",
        fontSize: `${(cfg.storeShopTitleFontSize ?? 15) > 0 ? cfg.storeShopTitleFontSize : 15}px`,
        fontWeight: String(cfg.storeShopTitleFontBold ?? 700) as any
    };

    const themeColor = cfg.themeColor || "#ff4d4f";
    const cssVars = {
        "--dec-search-result-bg": cfg.backgroundColor || "#f5f5f5",
        "--dec-search-item-bg": cfg.searchItemBackgroundColor || "#ffffff",
        "--dec-search-item-padding-lr": `${cfg.searchItemPaddingLeftRight ?? 10}px`,
        "--dec-search-result-item-bg": cfg.itemBackgroundColor || "#ffffff",
        "--dec-search-result-theme": themeColor,
        "--dec-search-result-theme-50": toRgba(themeColor, 0.15),
        "--dec-search-result-coupon-line": toRgba(themeColor, 0.35),
        "--dec-search-result-price-color": cfg.priceColor || themeColor,
        "--dec-search-result-margin": `${cfg.margin ?? 0}px`,
        "--dec-search-result-padding-tb": `${cfg.paddingTopBottom ?? cfg.padding ?? 10}px`,
        "--dec-search-result-padding-lr": `${cfg.paddingLeftRight ?? cfg.padding ?? 10}px`,
        "--dec-search-result-image-radius": `${cfg.productImageRadius ?? 4}px`,
        "--dec-product-title-line": cfg.productTitleLine || 2,
        // tag-bg.png 本地资源缺失：以主题渐变承接（原版为 url(tag-bg.png)）
        "--dec-store-tag-bg": `linear-gradient(90deg, ${toRgba(themeColor, 0.85)}, ${themeColor})`
    } as CSSProperties;

    return (
        <div className="module-ad-con dec-mobile-search-result module-search-result" style={cssVars}>
            <div className="dec-mobile-search-result__inner">
                <PageHeaderPreview variant={headerLayoutStyle === 1 ? "domestic" : "overseas"} cartCount={headerLayoutStyle === 2 ? "21" : ""} />

                {headerLayoutStyle === 1 ? (
                    <div className="dec-mobile-search-result__header">
                        <div className="dec-mobile-search-result__input-wrapper">
                            <div className="dec-mobile-search-result__input-area">
                                <span className="dec-mobile-search-result__search-text">卫衣</span>
                            </div>
                            <div className="dec-mobile-search-result__search-btn">搜索</div>
                        </div>
                    </div>
                ) : (
                    <div className="dec-mobile-search-result__header dec-mobile-search-result__header--style2">
                        <span className="dec-mobile-search-result__back-icon">
                            <ChevronLeft size={14} />
                        </span>
                        <div className="dec-mobile-search-result__input-wrapper">
                            <div className="dec-mobile-search-result__input-area">
                                <span className="dec-mobile-search-result__search-text">卫衣</span>
                            </div>
                            <span className="dec-mobile-search-result__search-icon-right">
                                <SearchIcon size={18} />
                            </span>
                        </div>
                    </div>
                )}

                {headerLayoutStyle === 1 && !isStoreResult ? (
                    <div className="dec-mobile-search-result__tabs">
                        <div className="dec-mobile-search-result__tab-item active">
                            <span>默认</span>
                        </div>
                        <div className="dec-mobile-search-result__tab-item">
                            <span>销量</span>
                        </div>
                        <div className="dec-mobile-search-result__tab-item dec-mobile-search-result__tab-item--price">
                            <span>价格</span>
                            <div className="dec-mobile-search-result__price-icons">
                                <Triangle size={8} up />
                                <Triangle size={8} />
                            </div>
                        </div>
                        <div className="dec-mobile-search-result__tab-item dec-mobile-search-result__tab-item--filter">
                            <span>筛选</span>
                            <span className="dec-mobile-search-result__filter-arrow">
                                <ChevronDown size={8} />
                            </span>
                        </div>
                    </div>
                ) : null}

                {headerLayoutStyle === 2 && !isStoreResult ? (
                    <div className="dec-mobile-search-result__tabs dec-mobile-search-result__tabs--style2">
                        <div className="dec-mobile-search-result__tabs-scroll">
                            <div className="dec-mobile-search-result__tab-item-box">
                                <span className="dec-mobile-search-result__filter-ico">
                                    <FilterIcon size={12} />
                                </span>
                                <span>筛选</span>
                            </div>
                            {OVERSEAS_FILTER_TABS.map((name) => (
                                <div key={name} className="dec-mobile-search-result__tab-item-box">
                                    <span>{name}</span>
                                    <span className="dec-mobile-search-result__tab-arrow">
                                        <ChevronDown size={10} />
                                    </span>
                                </div>
                            ))}
                        </div>
                        <div className="dec-mobile-search-result__tabs-more">
                            <span className="dec-mobile-search-result__tab-arrow">
                                <ChevronDown size={14} />
                            </span>
                        </div>
                    </div>
                ) : null}

                {isStoreResult ? (
                    <div
                        className="dec-mobile-search-result__content dec-mobile-search-result__content--store"
                        style={{ rowGap: `${storeGap}px` }}
                    >
                        {mockStores.map((store) => (
                            <div key={store.id} className="dec-mobile-search-result__store" style={storeCardStyle}>
                                <div className="dec-mobile-search-result__store-header">
                                    <div className="dec-mobile-search-result__store-logo" style={{ borderRadius: `${storeLogoRadius}%` }}>
                                        <img src={store.logo} alt={store.title} />
                                    </div>
                                    <div className="dec-mobile-search-result__store-main">
                                        <div className="dec-mobile-search-result__store-title-row">
                                            {storeVisible.showRating ? <span className="dec-mobile-search-result__store-tag">星级门店</span> : null}
                                            <span className="dec-mobile-search-result__store-title" style={storeTitleStyle}>
                                                {store.title}
                                            </span>
                                            {storeVisible.showFollow ? (
                                                <span className="dec-mobile-search-result__store-follow">
                                                    <HeartIcon size={11} />
                                                    关注
                                                </span>
                                            ) : null}
                                        </div>
                                        <div className="dec-mobile-search-result__store-meta">
                                            {storeVisible.showRating ? (
                                                <span className="dec-mobile-search-result__store-score">{store.score}分</span>
                                            ) : null}
                                            {storeVisible.showMinOrder ? (
                                                <span className="dec-mobile-search-result__store-meta-item">起送 ¥{store.minOrder}</span>
                                            ) : null}
                                            {storeVisible.showDeliveryFee ? (
                                                <span className="dec-mobile-search-result__store-meta-item">配送费 ¥{store.deliveryFee}</span>
                                            ) : null}
                                            {storeVisible.showDistance ? (
                                                <span className="dec-mobile-search-result__store-meta-item">距离 {store.distance}</span>
                                            ) : null}
                                        </div>
                                    </div>
                                </div>
                                <div className="dec-mobile-search-result__store-products" style={{ columnGap: `${storeProductGap}px` }}>
                                    {store.products.map((item) => (
                                        <div key={item.id} className="dec-mobile-search-result__store-product">
                                            <div className="dec-mobile-search-result__store-product-image">
                                                <img src={item.image} alt={item.title} />
                                            </div>
                                            {storeVisible.showProductName ? (
                                                <div
                                                    className={`dec-mobile-search-result__store-product-name dec-mobile-search-result__store-product-name--line${storeProductNameRow}`}
                                                >
                                                    {item.title}
                                                </div>
                                            ) : null}
                                            {storeVisible.showProductPrice ? (
                                                <div className="dec-mobile-search-result__store-product-price">¥{item.price}</div>
                                            ) : null}
                                        </div>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                ) : (
                    <div className={`dec-mobile-search-result__content dec-mobile-search-result__content--list${cfg.listStyle}`}>
                        {mockProducts.map((item) => (
                            <div key={item.id} className="dec-mobile-search-result__item">
                                <div className="dec-mobile-search-result__item-image">
                                    <img src={item.image} alt={item.title} />
                                </div>
                                <div className="dec-mobile-search-result__item-info">
                                    {cfg.listStyle === 2 ? (
                                        <>
                                            <div className="dec-mobile-search-result__item-title" style={{ color: cfg.productTitleColor }}>
                                                {item.title}
                                            </div>
                                            {cfg.showSales !== false ? (
                                                <div className="dec-mobile-search-result__item-sales">已售 {item.quantity || 0}</div>
                                            ) : null}
                                            <div className="dec-mobile-search-result__item-action">
                                                <div className="dec-mobile-search-result__item-price">
                                                    <span className="dec-mobile-search-result__price-symbol">¥</span>
                                                    <span className="dec-mobile-search-result__price-value">{item.price}</span>
                                                </div>
                                                {cfg.showShopTag !== false ? (
                                                    <div className="dec-mobile-search-result__item-shop">官方自营店</div>
                                                ) : null}
                                                {cfg.showCartButton !== false ? (
                                                    <div className="dec-mobile-search-result__item-cart">
                                                        <div className="dec-mobile-search-result__cart-btn">添加购物车</div>
                                                    </div>
                                                ) : null}
                                            </div>
                                        </>
                                    ) : (
                                        <>
                                            <div className="dec-mobile-search-result__item-title" style={{ color: cfg.productTitleColor }}>
                                                {item.title}
                                            </div>
                                            {cfg.showCoupon !== false ? (
                                                <div className="dec-mobile-search-result__item-coupon">
                                                    <span className="dec-mobile-search-result__coupon-type">
                                                        券
                                                        <i className="dec-mobile-search-result__coupon-line" />
                                                    </span>
                                                    <span className="dec-mobile-search-result__coupon-content">满 100.00 减 10.00</span>
                                                </div>
                                            ) : null}
                                            <div className="dec-mobile-search-result__item-bottom">
                                                <div className="dec-mobile-search-result__item-price">
                                                    <span className="dec-mobile-search-result__price-symbol">¥</span>
                                                    <span className="dec-mobile-search-result__price-value">{item.price}</span>
                                                </div>
                                            </div>
                                            {buyBtnStyle > 0 ? (
                                                <div className="dec-mobile-search-result__buy-btn-box">
                                                    <a
                                                        href="javascript:;"
                                                        className={`dec-mobile-search-result__buy-btn dec-mobile-search-result__buy-btn--style${buyBtnStyle}`}
                                                        onClick={(e) => e.preventDefault()}
                                                    >
                                                        {buyBtnStyle === 5 || buyBtnStyle === 6 ? (
                                                            <span>购买</span>
                                                        ) : buyBtnStyle === 7 || buyBtnStyle === 8 ? (
                                                            <span>马上抢</span>
                                                        ) : buyBtnStyle >= 1 && buyBtnStyle <= 4 ? (
                                                            <BuyBtnIcon style={buyBtnStyle} />
                                                        ) : null}
                                                    </a>
                                                </div>
                                            ) : null}
                                        </>
                                    )}
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
