/**
 * 装修编辑器模块图标总表（单一源，2026-11-15 合并）：
 * 此前 PC 编辑器有 LIB_META（PcDecorateEditor.tsx 内，约 20 个 pc-* 型）、
 * H5 编辑器有 MODULE_ICONPARK（decorateEditorH5/components/moduleIcons.tsx，约 70 个移动型），
 * 两张表同为 IconPark 图标语言却各存一份。现合并到本文件：
 * - MODULE_ICONPARK：移动端模块 type → 图标（H5 模块库用）
 * - PC_LIB_META：PC 模块 type → { order, Icon }（PC 模块库排序+图标用）
 * - EDITOR_MODULE_ICONS：两表按 type 合并的统一视图（type 集合不重叠：PC 为 pc-* 前缀、
 *   移动端为无前缀 type，故同 type 必然同图标，不存在覆盖冲突）
 * 新增模块时只在本文件补一行，两端同步生效；禁止再在各编辑器内另建图标表。
 */
import type { Icon as IconParkIcon } from "@icon-park/react/es/runtime";
import Ad from "@icon-park/react/es/icons/Ad";
import AllApplication from "@icon-park/react/es/icons/AllApplication";
import Announcement from "@icon-park/react/es/icons/Announcement";
import ApplicationMenu from "@icon-park/react/es/icons/ApplicationMenu";
import ApplicationTwo from "@icon-park/react/es/icons/ApplicationTwo";
import BookOne from "@icon-park/react/es/icons/BookOne";
import Carousel from "@icon-park/react/es/icons/Carousel";
import ClickTap from "@icon-park/react/es/icons/ClickTap";
import Commodity from "@icon-park/react/es/icons/Commodity";
import Compass from "@icon-park/react/es/icons/Compass";
import Components from "@icon-park/react/es/icons/Components";
import Coupon from "@icon-park/react/es/icons/Coupon";
import DividingLine from "@icon-park/react/es/icons/DividingLine";
import Fire from "@icon-park/react/es/icons/Fire";
import FullScreen from "@icon-park/react/es/icons/FullScreen";
import Gift from "@icon-park/react/es/icons/Gift";
import GridFour from "@icon-park/react/es/icons/GridFour";
import GridNine from "@icon-park/react/es/icons/GridNine";
import H1 from "@icon-park/react/es/icons/H1";
import HeadsetOne from "@icon-park/react/es/icons/HeadsetOne";
import ImageFiles from "@icon-park/react/es/icons/ImageFiles";
import LayoutOne from "@icon-park/react/es/icons/LayoutOne";
import Lightning from "@icon-park/react/es/icons/Lightning";
import ListSuccess from "@icon-park/react/es/icons/ListSuccess";
import MessageOne from "@icon-park/react/es/icons/MessageOne";
import MessageSearch from "@icon-park/react/es/icons/MessageSearch";
import MultiPictureCarousel from "@icon-park/react/es/icons/MultiPictureCarousel";
import Navigation from "@icon-park/react/es/icons/Navigation";
import Notebook from "@icon-park/react/es/icons/Notebook";
import PageTemplate from "@icon-park/react/es/icons/PageTemplate";
import Peoples from "@icon-park/react/es/icons/Peoples";
import Percentage from "@icon-park/react/es/icons/Percentage";
import Picture from "@icon-park/react/es/icons/Picture";
import PlayOne from "@icon-park/react/es/icons/PlayOne";
import RowHeight from "@icon-park/react/es/icons/RowHeight";
import Search from "@icon-park/react/es/icons/Search";
import Shop from "@icon-park/react/es/icons/Shop";
import ShoppingBagOne from "@icon-park/react/es/icons/ShoppingBagOne";
import Star from "@icon-park/react/es/icons/Star";
import TagOne from "@icon-park/react/es/icons/TagOne";
import Text from "@icon-park/react/es/icons/Text";
import Timer from "@icon-park/react/es/icons/Timer";
import User from "@icon-park/react/es/icons/User";
import Video from "@icon-park/react/es/icons/Video";
import ViewGridCard from "@icon-park/react/es/icons/ViewGridCard";
import ViewGridList from "@icon-park/react/es/icons/ViewGridList";
import WalletOne from "@icon-park/react/es/icons/WalletOne";

/** 移动端模块图标（原 decorateEditorH5/components/moduleIcons.tsx，内容不变） */
export const MODULE_ICONPARK: Record<string, IconParkIcon> = {
    imageAd: Picture,
    imageAd2: Picture,
    banner: Carousel,
    bannerA2: Carousel,
    slideLargeImage: FullScreen,
    slidingImage: MultiPictureCarousel,
    imageSquareAd: GridFour,
    imageShowcase: GridNine,
    imageHotarea: ClickTap,
    imageNav: ImageFiles,
    imageTextComponent: PageTemplate,
    article: BookOne,
    notice: Announcement,
    noticeA2: Announcement,
    noticeA3: Announcement,
    mixedNotice: Announcement,
    whiteLine: DividingLine,
    whiteBlank: RowHeight,
    titleAd: Text,
    commonTitle: H1,
    navbar: Compass,
    tagNav: TagOne,
    catNav: ViewGridList,
    categoryNav: ViewGridList,
    categoryNavA1: ViewGridList,
    categoryNavA2: ViewGridList,
    categoryNavA3: ViewGridList,
    categoryRec: ViewGridCard,
    categoryRecA1: ViewGridCard,
    cateCenterV1: AllApplication,
    cateCenterV2: AllApplication,
    product: Commodity,
    product2: Commodity,
    productA1: Commodity,
    productDetailA1: Commodity,
    productDetailA2: Commodity,
    productDetailA3: Commodity,
    productGrouping: ShoppingBagOne,
    productSlide: LayoutOne,
    productCarousel: MultiPictureCarousel,
    groupSliding: Components,
    comboSuggestions: ApplicationTwo,
    mixedProductDisplay: ApplicationMenu,
    searchBar: Search,
    searchBar2: Search,
    mobileSearchResult: MessageSearch,
    countdown: Timer,
    coupon: Coupon,
    seckill: Lightning,
    seckill2: Lightning,
    groupon: Peoples,
    live: PlayOne,
    videoNumber: Video,
    shopInfo: Shop,
    storeNav: Navigation,
    storeOrderCard: ListSuccess,
    shopTips: MessageOne,
    limitedSpecial: Percentage,
    specialOffer: Fire,
    newcomerCombo: Gift,
    adImage: Ad,
    account: User,
    userInfo: User,
    userInfoV2: User,
    userInfo3: User,
    mobileLogin: User,
    myWallet: WalletOne,
    myWalletV2: WalletOne,
    orderEntry: Notebook,
    orderEntryV2: Notebook,
    myService: HeadsetOne,
    fixedFunctionalArea: ApplicationMenu
};

/**
 * PC 模块库元数据（原 PcDecorateEditor.tsx LIB_META，内容不变）：
 * 原版 index.ts 用 alicdn 远程 iconfont 类名，但该字体在 admin-semi 里从未真正加载，
 * 故改用等效语义的 @icon-park/react 本地 SVG 组件（映射沿革见各行图标名）。
 */
export const PC_LIB_META: Record<string, { order: number; Icon: IconParkIcon }> = {
  pcLogin: { order: 1, Icon: ViewGridList },
  'pc-banner2': { order: 20, Icon: Carousel },
  'pc-group-banner': { order: 21, Icon: MultiPictureCarousel },
  'pc-home-hero': { order: 40, Icon: LayoutOne },
  'pc-category-nav': { order: 30, Icon: ViewGridList },
  'pc-category-nav3': { order: 31, Icon: ViewGridCard },
  'pc-home-quick-nav': { order: 32, Icon: ApplicationMenu },
  'pc-image-ad': { order: 22, Icon: Picture },
  'pc-image-nav': { order: 33, Icon: ImageFiles },
  'pc-image-square-ad': { order: 23, Icon: GridFour },
  'pc-image-hotarea': { order: 24, Icon: ClickTap },
  'pc-product': { order: 10, Icon: Commodity },
  'pc-product-group': { order: 11, Icon: ShoppingBagOne },
  'pc-recommend-product': { order: 12, Icon: Star },
  'pc-divider': { order: 51, Icon: DividingLine },
  'pc-spacer': { order: 52, Icon: RowHeight },
  'pc-notice': { order: 44, Icon: Announcement },
  'pc-title-ad': { order: 50, Icon: Text },
  'pc-home-coupon': { order: 41, Icon: Coupon },
  'pc-home-groupon': { order: 42, Icon: Peoples },
  'pc-home-seckill': { order: 43, Icon: Lightning },
  pcBanner: { order: 25, Icon: Carousel },
  pcBanner2: { order: 26, Icon: Carousel },
  pcGroupBanner: { order: 27, Icon: MultiPictureCarousel },
  pcImageAd: { order: 28, Icon: Picture },
  pcCustomAd1: { order: 29, Icon: LayoutOne },
  pcCustomAd2: { order: 30, Icon: LayoutOne },
  pcCategoryNav: { order: 34, Icon: ViewGridList },
  pcCategoryNav3: { order: 35, Icon: ViewGridCard },
  pcCatWide: { order: 16, Icon: ViewGridCard },
  pcCatProductNormal: { order: 13, Icon: Commodity },
  pcCatProductWide: { order: 14, Icon: Commodity },
  pcCatProductSimple: { order: 15, Icon: ShoppingBagOne },
  pcCoupon: { order: 46, Icon: Coupon },
  pcGroupon: { order: 47, Icon: Peoples },
  pcSeckill: { order: 45, Icon: Lightning },
  pcRecommendProduct: { order: 17, Icon: Star },
};

/** 统一视图：任意编辑器模块 type → 图标（两表 type 集合不重叠，合并无覆盖） */
export const EDITOR_MODULE_ICONS: Record<string, IconParkIcon> = {
  ...MODULE_ICONPARK,
  ...Object.fromEntries(Object.entries(PC_LIB_META).map(([type, meta]) => [type, meta.Icon])),
};
