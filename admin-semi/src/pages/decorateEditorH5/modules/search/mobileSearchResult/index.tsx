/**
 * 搜索结果 模块（契约依据：uniapp-react components/modules/search-result/config.ts 的 searchResultDefaults，
 * 该文件注明必须与后台 mobileSearch/mobileSearchResult/types.ts 的 mobileSearchResultDefaults 完全一致；
 * placeholder 为搜索页 pages/search/index.tsx 的搜索框提示字段，generic 现注册 defaults 亦含此字段）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

export const mobileSearchResultDefaults = {
    placeholder: "点击搜索商品",
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
    platform: "mobile",
    order: 103,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(mobileSearchResultDefaults)
});
