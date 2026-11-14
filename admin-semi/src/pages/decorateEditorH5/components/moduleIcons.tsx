/**
 * 模块库图标（IconPark）：按模块 type 映射，与 PC 编辑器 LIB_META 同一套图标语言。
 * 未映射的 type 由 ModuleToolbar 回退到注册时自带的 icon（历史 emoji HTML）。
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
import Components from "@icon-park/react/es/icons/Components";
import Coupon from "@icon-park/react/es/icons/Coupon";
import DividingLine from "@icon-park/react/es/icons/DividingLine";
import Fire from "@icon-park/react/es/icons/Fire";
import FullScreen from "@icon-park/react/es/icons/FullScreen";
import Gift from "@icon-park/react/es/icons/Gift";
import GridFour from "@icon-park/react/es/icons/GridFour";
import GridNine from "@icon-park/react/es/icons/GridNine";
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
import TagOne from "@icon-park/react/es/icons/TagOne";
import Text from "@icon-park/react/es/icons/Text";
import Timer from "@icon-park/react/es/icons/Timer";
import User from "@icon-park/react/es/icons/User";
import Video from "@icon-park/react/es/icons/Video";
import ViewGridCard from "@icon-park/react/es/icons/ViewGridCard";
import ViewGridList from "@icon-park/react/es/icons/ViewGridList";
import WalletOne from "@icon-park/react/es/icons/WalletOne";

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
    customProduct: Commodity,
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
