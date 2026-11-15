/**
 * 搜索结果 模块（契约自 view/admin editor/modules/mobileSearch/mobileSearchResult/index.ts + types.ts 逐字段移植）
 *
 * 注册口径：type "mobileSearchResult"、label「搜索结果」、url "mobileSearch"、
 * platform "mobile-search"、toolbarGroup "searchResult"、order 80、exclusive: true
 * （搜索页整页只允许一个）。原版 index.ts 无 enabled 门控，故不加；
 * 门店分支的版本门控在 Settings 内以 isStore() 承接（见 Settings 注释）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

/** 与原版 types.ts 的 mobileSearchResultDefaults 逐字段一致（契约，共 39 字段） */
export const mobileSearchResultDefaults = {
    searchResultType: 1,
    backgroundColor: "#f5f5f5",
    searchItemBackgroundColor: "#ffffff",
    searchItemPaddingLeftRight: 10,
    itemBackgroundColor: "#ffffff",
    themeColor: "#ff4d4f",
    showShopTag: true,
    showSales: true,
    showCartButton: true,
    showCoupon: true,
    layoutStyle: 1,
    listStyle: 1,
    productTitleColor: "#333333",
    productTitleLine: 2,
    priceColor: "#ff4d4f",
    buyBtnStyle: 1,
    margin: 0,
    padding: 10,
    paddingTopBottom: 10,
    paddingLeftRight: 10,
    moduleRadius: 0,
    productImageRadius: 4,
    storeShowRating: 1,
    storeShowMinOrder: 1,
    storeShowDeliveryFee: 1,
    storeShowDistance: 1,
    storeShowFollow: 1,
    storeShowProductName: 1,
    storeProductNameRow: 1,
    storeShowProductPrice: 1,
    storeProductGap: 8,
    storeGap: 10,
    storePaddingTopBottom: 12,
    storePaddingLeftRight: 12,
    storeCardRadius: 8,
    storeLogoRadius: 50,
    storeShopTitleColor: "#1a1a1a",
    storeShopTitleFontSize: 15,
    storeShopTitleFontBold: 700
};

registerModule({
    type: "mobileSearchResult",
    label: "搜索结果",
    icon: '<i class="h5-mod-ico">🔍</i>',
    url: "mobileSearch",
    platform: "mobile-search",
    toolbarGroup: "searchResult",
    order: 80,
    exclusive: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(mobileSearchResultDefaults)
});
