/**
 * 商品分组 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/productGrouping）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultProducts, defaultBuyButtonStyle } from "../../../contract/sharedDefaults";

export const productGroupingDefaults = {
    groupList: [
        { title: "全部", subTitle: "" },
        { title: "热销", subTitle: "HOT" },
        { title: "新品", subTitle: "NEW" }
    ],
    productGroups: [
        { title: "热销", subTitle: "HOT", products: cloneDeep(defaultProducts) },
        { title: "新品", subTitle: "NEW", products: cloneDeep(defaultProducts) }
    ],
    products: cloneDeep(defaultProducts),
    productSource: cloneDeep(defaultProducts),
    groupTitle: {
        showTitle: 1,
        showSubTitle: 1,
        titleColor: "#333333",
        activeTitleColor: "#e1251b",
        subTitleColor: "#999999",
        activeSubTitleColor: "#ffffff",
        titleFontSize: 15,
        bigTitleFontSize: 18,
        subTitleRadius: 10,
        subTitleGradientType: "leftRight",
        gradientColorA: "#FF7A45",
        gradientColorB: "#FF3630",
        splitLineColor: "#eeeeee",
        marginLeftRight: 10
    },
    productCardStyle: 1,
    cardMargin: 8,
    cardPadding: 8,
    cardRadius: 8,
    crossedOutPrice: 1,
    picFillType: "cover",
    picScale: "1:1",
    picBottomRadius: 1,
    productNameRow: 2,
    productNameSize: 13,
    productNameWeight: 400,
    productPriceSize: 15,
    productPriceWeight: 600,
    productPriceMarginTop: 4,
    productPricePic: { picUrl: "", picThumb: "" },
    showProductName: 1,
    showProductPrice: 1,
    bottomMoreBtn: 0,
    bottomMoreBtnText: "查看更多",
    buyBtnStyle: cloneDeep(defaultBuyButtonStyle),
    productColor: {
        backgroundColor: "#ffffff",
        titleColor: "#333333",
        priceColor: "#e1251b",
        crossedOutPriceColor: "#999999",
        moreBtnBgColor: "#f5f5f5",
        moreBtnBorderColor: "#eeeeee",
        moreBtnTextColor: "#333333"
    },
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "productGrouping",
    label: "商品分组",
    icon: '<i class="h5-mod-ico">🗃️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 62,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productGroupingDefaults)
});
