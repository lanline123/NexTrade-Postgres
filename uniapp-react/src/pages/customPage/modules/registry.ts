/**
* 装修模块注册表：module.type → React 组件。
* 模块按迁移批次陆续补入（对应旧 modules/index.vue 的 40+ 分支），
* 未注册的类型渲染 null（DevTools 可见 TODO 日志），不影响其余模块。
*
*
* 原版 uni+Vite 经 @uni-ku/bundle-optimizer 把装修模块下沉为异步分块，首页壳
* 分块仅 2.9KB gz；我方原静态全量导入使全部模块打进页面分块（首页 568 号
* 分块原始 829KB，串行第二轮才到）。此处改为 React.lazy + 动态 import：
* 每个模块独立成块、按需并行加载，页面分块只剩分发壳。渲染侧
* （DecorateModules / ShopModules）以 <Suspense fallback={null}> 包裹，
* 与「未注册渲染 null」的既有口径一致，模块到达即上屏。
*/
import { lazy} from "react";
import type { ComponentType} from "react";

export interface DecorateModuleProps {
module: any;
moduleIndex?: number;
index?: number;
scrollTop?: number;
needStatusBar?: boolean;
onChangeCatNav?: (data: any) => void;
onIsCut?: (val: boolean) => void;
/** 店铺上下文（原版店铺页分发口径）：SHOP_ID 类模块与 shopInfo 模块才收到 */
shopId?: number;
shopInfo?: import("@/types/shop/shop").ShopDetailItem;
onRefreshShopDetail?: (shopId?: number) => void;
}

type ModuleLoader = () => Promise<{ default: ComponentType<DecorateModuleProps>}>;

const lazyModule = (loader: ModuleLoader): ComponentType<DecorateModuleProps> =>
lazy(loader) as unknown as ComponentType<DecorateModuleProps>;

export const MODULE_REGISTRY: Record<string, ComponentType<DecorateModuleProps>> = {
navbar: lazyModule(() => import(/* webpackChunkName: "mod-navbar" */ "./navbar/index")),
searchBar: lazyModule(() => import(/* webpackChunkName: "mod-searchBar" */ "./searchBar/index")),
searchBar2: lazyModule(() => import(/* webpackChunkName: "mod-searchBar2" */ "./searchBar2/index")),
notice: lazyModule(() => import(/* webpackChunkName: "mod-notice" */ "./notice/index")),
noticeA2: lazyModule(() => import(/* webpackChunkName: "mod-noticeA2" */ "./noticeA2/index")),
noticeA3: lazyModule(() => import(/* webpackChunkName: "mod-noticeA3" */ "./noticeA3/index")),
mixedNotice: lazyModule(() => import(/* webpackChunkName: "mod-mixedNotice" */ "./mixedNotice/index")),
shopTips: lazyModule(() => import(/* webpackChunkName: "mod-shopTips" */ "./shopTips/index")),
tagNav: lazyModule(() => import(/* webpackChunkName: "mod-tagNav" */ "./tagNav/index")),
whiteBlank: lazyModule(() => import(/* webpackChunkName: "mod-whiteBlank" */ "./whiteBlank/index")),
whiteLine: lazyModule(() => import(/* webpackChunkName: "mod-whiteLine" */ "./whiteLine/index")),
commonTitle: lazyModule(() => import(/* webpackChunkName: "mod-commonTitle" */ "./commonTitle/index")),
catNav: lazyModule(() => import(/* webpackChunkName: "mod-catNav" */ "./catNav/index")),
titleAd: lazyModule(() => import(/* webpackChunkName: "mod-titleAd" */ "./titleAd/index")),
imageAd: lazyModule(() => import(/* webpackChunkName: "mod-imageAd" */ "./imageAd/index")),
imageNav: lazyModule(() => import(/* webpackChunkName: "mod-imageNav" */ "./imageNav/index")),
imageSquareAd: lazyModule(() => import(/* webpackChunkName: "mod-imageSquareAd" */ "./imageSquareAd/index")),
imageHotarea: lazyModule(() => import(/* webpackChunkName: "mod-imageHotarea" */ "./imageHotarea/index")),
product: lazyModule(() => import(/* webpackChunkName: "mod-product" */ "./product/index")),
product2: lazyModule(() => import(/* webpackChunkName: "mod-product2" */ "./product2/index")),
productA1: lazyModule(() => import(/* webpackChunkName: "mod-productA1" */ "./productA1/index")),
banner: lazyModule(() => import(/* webpackChunkName: "mod-banner" */ "./banner/index")),
bannerA2: lazyModule(() => import(/* webpackChunkName: "mod-bannerA2" */ "./bannerA2/index")),
imageAd2: lazyModule(() => import(/* webpackChunkName: "mod-imageAd2" */ "./imageAd2/index")),
slidingImage: lazyModule(() => import(/* webpackChunkName: "mod-slidingImage" */ "./slidingImage/index")),
slideLargeImage: lazyModule(() => import(/* webpackChunkName: "mod-slideLargeImage" */ "./slideLargeImage/index")),
imageShowcase: lazyModule(() => import(/* webpackChunkName: "mod-imageShowcase" */ "./imageShowcase/index")),
productSlide: lazyModule(() => import(/* webpackChunkName: "mod-productSlide" */ "./productSlide/index")),
productCarousel: lazyModule(() => import(/* webpackChunkName: "mod-productCarousel" */ "./productCarousel/index")),
limitedSpecial: lazyModule(() => import(/* webpackChunkName: "mod-limitedSpecial" */ "./limitedSpecial/index")),
specialOffer: lazyModule(() => import(/* webpackChunkName: "mod-specialOffer" */ "./specialOffer/index")),
coupon: lazyModule(() => import(/* webpackChunkName: "mod-coupon" */ "./coupon/index")),
seckill: lazyModule(() => import(/* webpackChunkName: "mod-seckill" */ "./seckill/index")),
seckill2: lazyModule(() => import(/* webpackChunkName: "mod-seckill2" */ "./seckill2/index")),
countdown: lazyModule(() => import(/* webpackChunkName: "mod-countdown" */ "./countdown/index")),
groupSliding: lazyModule(() => import(/* webpackChunkName: "mod-groupSliding" */ "./groupSliding/index")),
productGrouping: lazyModule(() => import(/* webpackChunkName: "mod-productGrouping" */ "./productGrouping/index")),
comboSuggestions: lazyModule(() => import(/* webpackChunkName: "mod-comboSuggestions" */ "./comboSuggestions/index")),
mixedProductDisplay: lazyModule(() => import(/* webpackChunkName: "mod-mixedProductDisplay" */ "./mixedProductDisplay/index")),
newcomerCombo: lazyModule(() => import(/* webpackChunkName: "mod-newcomerCombo" */ "./newcomerCombo/index")),
groupon: lazyModule(() => import(/* webpackChunkName: "mod-groupon" */ "./groupon/index")),
categoryNav: lazyModule(() => import(/* webpackChunkName: "mod-categoryNav" */ "./categoryNav/index")),
categoryNavA1: lazyModule(() => import(/* webpackChunkName: "mod-categoryNavA1" */ "./categoryNavA1/index")),
categoryNavA2: lazyModule(() => import(/* webpackChunkName: "mod-categoryNavA2" */ "./categoryNavA2/index")),
categoryNavA3: lazyModule(() => import(/* webpackChunkName: "mod-categoryNavA3" */ "./categoryNavA3/index")),
categoryRec: lazyModule(() => import(/* webpackChunkName: "mod-categoryRec" */ "./categoryRec/index")),
categoryRecA1: lazyModule(() => import(/* webpackChunkName: "mod-categoryRecA1" */ "./categoryRecA1/index")),
imageTextComponent: lazyModule(() => import(/* webpackChunkName: "mod-imageTextComponent" */ "./imageTextComponent/index")),
// 仅微信小程序端生效（旧 #ifdef MP-WEIXIN，非 weapp 渲染 null）
videoNumber: lazyModule(() => import(/* webpackChunkName: "mod-videoNumber" */ "./videoNumber/index")),
live: lazyModule(() => import(/* webpackChunkName: "mod-live" */ "./live/index")),
shopInfo: lazyModule(() => import(/* webpackChunkName: "mod-shopInfo" */ "./shopInfo/index")),
// 四个模块均有原版编辑器源契约（article 在 mobile 组，余三者在 mobilePro 组）；
// 2026-11-18 补齐：编辑器侧已全部可放置（article/customProduct 放开 visible、
// storeNav/storeOrderCard 解放置死锁），storeNav/storeOrderCard 渲染器按原版
// 编辑器契约重建（原版 uniapp 从未有这两个渲染器），契约见各自模块文件头
article: lazyModule(() => import(/* webpackChunkName: "mod-article" */ "./article/index")),
customProduct: lazyModule(() => import(/* webpackChunkName: "mod-customProduct" */ "./customProduct/index")),
storeNav: lazyModule(() => import(/* webpackChunkName: "mod-storeNav" */ "./storeNav/index")),
storeOrderCard: lazyModule(() => import(/* webpackChunkName: "mod-storeOrderCard" */ "./storeOrderCard/index"))
};

/** 挂载在 catNav 分支之外的 sticky 变体（旧 catNav/sticky.vue） */
export const STICKY_REGISTRY: Record<string, ComponentType<DecorateModuleProps>> = {
catNav: lazyModule(() => import(/* webpackChunkName: "mod-catNavSticky" */ "./catNav/Sticky"))
};
