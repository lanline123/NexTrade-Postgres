/**
 * 商品详情模块默认值（移植自 ref-editor mobileProductDetail/_shared/defaults.ts + types.ts）
 * 三个风格共享同一套字段，仅 decorateStyle 与少量覆盖值不同。
 * 注：原版 _shared/ProductDetailPreview.vue 是简化骨架，三个风格的 Preview.vue 各自有完整模板——本移植以各风格 Preview 为准。
 */
import { cloneDeep } from "../../../contract/sharedDefaults";

export const productDetailBaseDefaults: Record<string, any> = {
    imageRatio: "4:3",
    imageMargin: 16,
    imageMode: 1,
    imageRadiusStyle: "square",
    titleSize: "medium",
    titleColor: "#1a1a1a",
    titleMaxLines: 2,
    priceColor: "#ff4d4f",
    marketPriceColor: "#999",
    // 顺序 = 页面自然序（券→活动→规格→评价→售后，2026-11-21 方案二接通消费
    // 后与前台 sortBlocks 口径对齐；此前为原版出厂序，前台视为未配置同样按
    // 自然序渲染，新数据直接写自然序使存量与生效口径字面一致）
    moduleSortList: [
        { key: "coupon", label: "领劵/优惠", visible: true },
        { key: "activity", label: "活动信息", visible: true },
        { key: "spec", label: "规格参数", visible: true },
        { key: "review", label: "评价入口", visible: true },
        { key: "afterSale", label: "售后入口", visible: true }
    ],
    buyBarStyle: 1,
    buttonRadius: 5,
    showHome: true,
    showService: true,
    showCart: true,
    showShop: true,
    backgroundColor: "#ffffff",
    contentBackgroundColor: "#ffffff"
};

export function createProductDetailDefaults(style: "A1" | "A2" | "A3"): Record<string, any> {
    const defaults: Record<string, any> = { ...cloneDeep(productDetailBaseDefaults), decorateStyle: style };
    if (style === "A1") {
        defaults.backgroundColor = "transparent";
        defaults.contentBackgroundColor = "#ffffff";
    }
    return defaults;
}

/** A2（门店风）：主图默认 1:1、边距 0、页面底色 #f5f5f5 */
export const productDetailA2Defaults: Record<string, any> = {
    ...createProductDetailDefaults("A2"),
    imageRatio: "1:1",
    imageMargin: 0,
    backgroundColor: "#f5f5f5"
};

/** A3（跨境风）：黑色价格、浅灰页面底色、白底卡片、显示商城头部 */
export const productDetailA3Defaults: Record<string, any> = {
    ...createProductDetailDefaults("A3"),
    imageRatio: "1:1",
    imageMargin: 16,
    imageRadiusStyle: "square",
    titleSize: "small",
    titleColor: "#1a1a1a",
    titleMaxLines: 2,
    priceColor: "#1a1a1a",
    buyBarStyle: 2,
    buttonRadius: 6,
    showHeader: true,
    backgroundColor: "#f5f5f5",
    contentBackgroundColor: "#ffffff"
};

/** 购买栏工具图标（原版 staticResource('product/*.png')，基址取原版模板硬编码的生产静态域名） */
export const productActionIcons = {
    home: "https://overseas.vortmall.cn/api/static/mini/images/product/home.png",
    service: "https://overseas.vortmall.cn/api/static/mini/images/product/service.png",
    cart: "https://overseas.vortmall.cn/api/static/mini/images/product/cart.png"
};

/** 详情示例图（原版 Preview 硬编码演示图） */
export const PD_DEMO_IMAGE =
    "https://oss.vortmall.com/upload/c4ca4238/img/item/demo/1680589052jGYXpahRm1SOtsPK8k!!pic800x800.jpeg?x-oss-process=image/resize,m_pad,h_800,h_800";
export const PD_DEMO_DESC_IMAGES = [
    "https://oss.vortmall.com/upload/c4ca4238/img/demo/desc/202304/1680589050BrWSyP9OxygOCIhonN!!pic.png",
    "https://oss.vortmall.com/upload/c4ca4238/img/demo/desc/202304/16805890506ZadzrY6JyGORlDwBW!!pic.png",
    "https://oss.vortmall.com/upload/c4ca4238/img/demo/desc/202304/16805890501AO5Y0RVcw3O1rnIZD!!pic.png",
    "https://oss.vortmall.com/upload/c4ca4238/img/demo/desc/202304/1680589050DBrEFcXHc1eSkUu1H1!!pic.png",
    "https://oss.vortmall.com/upload/c4ca4238/img/demo/desc/202304/16805890502KtJ7dRDutTfc2Esp5!!pic.png",
    "https://oss.vortmall.com/upload/c4ca4238/img/demo/desc/202304/1680589050fMuHuRcZVJ7wc03mMD!!pic.png"
];

export const pdImageRatioMap: Record<string, string> = { "4:3": "4 / 3", "4:5": "4 / 5", "1:1": "1 / 1" };
