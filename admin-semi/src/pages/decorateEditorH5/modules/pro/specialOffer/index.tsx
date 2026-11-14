/**
 * 特价专区 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/specialOffer；注意渲染端拼写 prodcutTitles/pruduct 保持原样）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle, defaultProducts } from "../../../contract/sharedDefaults";

export const specialOfferDefaults = {
    moduleTitle: {
        title: "特价专区",
        subTitle: "",
        buttonText: "更多",
        buttonLink: { link: "", title: "" },
        countdownTitle: "距结束",
        countdownType: "dailyCycle",
        dailyTime: [],
        specifyTime: 0
    },
    moduleColor: {
        titleColor: "#333333",
        subTitleColor: "#999999",
        countdownColor: "#e1251b",
        countdownTitleColor: "#333333",
        productNameColor: "#333333",
        sellingPointsColor: "#999999",
        buttonTextColor: "#e1251b"
    },
    moduleColorStyle: 1,
    productGroups: [
        { title: "特价组一", products: cloneDeep(defaultProducts) }
    ],
    productList: [],
    products: cloneDeep(defaultProducts),
    productSource: cloneDeep(defaultProducts),
    prodcutTitles: [],
    showProdcutTitles: 0,
    showProductName: 1,
    showProductPrice: 1,
    showSellingPoints: 0,
    productNameSize: 13,
    productNameWeight: 400,
    productRadius: 8,
    productWidth: 30,
    picFillType: "cover",
    picScale: "1:1",
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "specialOffer",
    label: "特价专区",
    icon: '<i class="h5-mod-ico">💰</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 67,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(specialOfferDefaults)
});
