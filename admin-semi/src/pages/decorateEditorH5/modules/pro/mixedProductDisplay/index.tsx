/**
 * 混合商品展示 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/mixedProductDisplay）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle, defaultProducts } from "../../../contract/sharedDefaults";

export const mixedProductDisplayDefaults = {
    productGroups: [
        { title: "精选好物", subTitle: "品质之选", sellingPoint: "正品保障", products: cloneDeep(defaultProducts) },
        { title: "超值特惠", subTitle: "限时直降", sellingPoint: "低价好物", products: cloneDeep(defaultProducts) }
    ],
    products: cloneDeep(defaultProducts),
    rightPic: { picUrl: "", picThumb: "" },
    showCountdown: 0,
    countdownType: "dailyCycle",
    dailyTime: [],
    specifyTime: 0,
    timeContent: ["天", "时", "分", "秒"],
    crossedOutPrice: 1,
    showProductPrice: 1,
    productRadius: 8,
    picFillType: "cover",
    picScale: "1:1",
    moduleColor: {
        titleColor: "#333333",
        sellingPointColor: "#FF6600",
        priceColor: "#e1251b",
        priceBackgroundColor: "#FFF1F1",
        crossedOutPriceColor: "#999999",
        lineColor: "#eeeeee",
        countdownColor: "#ffffff",
        countdownBackgroundColor: "#e1251b"
    },
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "mixedProductDisplay",
    label: "混合商品展示",
    icon: '<i class="h5-mod-ico">🧩</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 56,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(mixedProductDisplayDefaults)
});
