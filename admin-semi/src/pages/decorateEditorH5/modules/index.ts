/**
 * 模块自注册入口（副作用导入触发 registerModule）
 *
 * 与 view/admin editor/modules/index.ts 对齐。
 * 2026-11-14 对齐批次：原 46 个走 generic/simpleModules 占位的 type 已全部补齐
 * 专属 Preview+Settings 并在此正式注册（精确版 import 在 generic 之前，
 * simpleModules 的 reg() 对已注册 type 自动跳过）；另新增渲染端独有的
 * navbar / commonTitle 两 type。generic/simpleModules 仅作未知 type 兜底保留。
 */

// ==================== mobile 基础 ====================
import "./basic/imageAd";
import "./basic/imageAd2";
import "./basic/imageSquareAd";
import "./basic/imageHotarea";
import "./basic/product";
import "./basic/product2";
import "./basic/whiteLine";
import "./basic/whiteBlank";
import "./basic/imageNav";
import "./basic/titleAd";
import "./basic/notice";
import "./basic/searchBar";
import "./basic/searchBar2";
import "./basic/catNav";
// 渲染端 registry 独有、旧编辑器缺失的两个 type（2026-11-14 补注册）
import "./basic/navbar";
import "./basic/commonTitle";
// 不进工具栏（visible: false），仅保证老数据可渲染
import "./basic/article";

// ==================== mobilePromotion 营销 ====================
import "./promotion/coupon";
import "./promotion/seckill";
import "./promotion/seckill2";
import "./promotion/groupon";
import "./promotion/shopInfo";
import "./promotion/live";
import "./promotion/videoNumber";

// ==================== mobilePro 高级 ====================
import "./pro/imageTextComponent";
import "./pro/imageShowcase";
import "./pro/tagNav";
import "./pro/noticeA2";
import "./pro/noticeA3";
import "./pro/categoryNav";
import "./pro/categoryRec";
import "./pro/mixedNotice";
import "./pro/bannerA2";
import "./pro/customProduct";
import "./pro/banner";
import "./pro/categoryNavA1";
import "./pro/categoryNavA2";
import "./pro/categoryNavA3";
import "./pro/categoryRecA1";
import "./pro/comboSuggestions";
import "./pro/countdown";
import "./pro/groupSliding";
import "./pro/limitedSpecial";
import "./pro/mixedProductDisplay";
import "./pro/newcomerCombo";
import "./pro/productA1";
import "./pro/productCarousel";
import "./pro/productGrouping";
import "./pro/productSlide";
import "./pro/shopTips";
import "./pro/slideLargeImage";
import "./pro/slidingImage";
import "./pro/specialOffer";
// 渲染端暂无入口：visible:false，仅存量数据可打开/预览（见各模块 index.tsx 文件头）
import "./pro/storeNav";
import "./pro/storeOrderCard";

// ==================== mobileMember 会员页 ====================
import "./member/userInfo";
import "./member/userInfoV2";
import "./member/userInfo3";
import "./member/orderEntry";
import "./member/orderEntryV2";
import "./member/myWallet";
import "./member/myWalletV2";
import "./member/adImage";
import "./member/myService";
import "./member/account";
import "./member/fixedFunctionalArea";

// ==================== mobileCateCenter 分类中心 ====================
import "./cateCenter/cateCenterV1";
import "./cateCenter/cateCenterV2";

// ==================== mobileProductDetail 商品详情 ====================
import "./productDetail/productDetailA1";
import "./productDetail/productDetailA2";
import "./productDetail/productDetailA3";

// ==================== mobileLogin / mobileSearch ====================
import "./login/mobileLogin";
import "./search/mobileSearchResult";

// ==================== 通用模块工厂（兜底：未精确移植的未知 type 才走这里） ====================
import "./generic/simpleModules";
