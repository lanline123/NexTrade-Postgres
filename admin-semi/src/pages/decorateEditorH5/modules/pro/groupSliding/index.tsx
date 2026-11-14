/**
 * 分组滑动 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/groupSliding）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle, defaultProducts, defaultBuyButtonStyle } from "../../../contract/sharedDefaults";

export const groupSlidingDefaults = {
    groupList: [
        { title: "热销榜", subTitle: "大家都在买", products: cloneDeep(defaultProducts) },
        { title: "新品榜", subTitle: "抢先尝鲜", products: cloneDeep(defaultProducts) }
    ],
    products: cloneDeep(defaultProducts),
    groupRadius: 8,
    groupWidth: 85,
    crossedOutPrice: 1,
    productNameRow: 2,
    productNameSize: 13,
    productNameWeight: 400,
    productPicRadius: 8,
    productPicRatio: "1:1",
    productPicRightRadius: 0,
    productPriceMarginTop: 4,
    showProductLabel: 0,
    showProductName: 1,
    showProductPrice: 1,
    showRanking: 0,
    buyBtnStyle: cloneDeep(defaultBuyButtonStyle),
    moduleColor: {
        titleColor: "#333333",
        titleSize: 16,
        titlePic: { picUrl: "", picThumb: "" },
        subTitleColor: "#999999",
        subTitleSize: 12,
        productBackgroundColor: "#ffffff",
        productPriceColor: "#e1251b",
        crossedOutPriceColor: "#999999",
        labelBackgroundColor: "#e1251b",
        labelTextColor: "#ffffff"
    },
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "groupSliding",
    label: "分组滑动",
    icon: '<i class="h5-mod-ico">↔️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 51,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(groupSlidingDefaults)
});
