/**
 * 倒计时 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/countdown）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle, defaultProducts } from "../../../contract/sharedDefaults";

export const countdownDefaults = {
    countdownData: {
        countdownType: "dailyCycle",
        dailyTime: [],
        specifyTime: 0,
        showCountdown: 1,
        timeContent: ["天", "时", "分", "秒"],
        countdownBackgroundPic: { picUrl: "", picThumb: "" },
        countdownPicBottom: 0,
        marginTop: 0
    },
    moduleTitle: { title: "限时活动", subTitle: "", moreText: "更多" },
    moduleColor: {
        titleColor: "#333333",
        subTitleColor: "#999999",
        countdownColor: "#ffffff",
        tipsColor: "#999999",
        productBackgroundColor: "#ffffff",
        productNameColor: "#333333",
        productPriceColor: "#e1251b",
        crossedOutPriceColor: "#999999"
    },
    pruductList: [],
    products: cloneDeep(defaultProducts),
    rowNum: 3,
    showProductName: 1,
    showProductPrice: 1,
    crossedOutPrice: 1,
    productPadding: 8,
    productRadius: 8,
    picFillType: "cover",
    picScale: "1:1",
    picBottomRadius: 1,
    priceWeight: 600,
    textAlignment: "left",
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "countdown",
    label: "倒计时",
    icon: '<i class="h5-mod-ico">⏰</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 49,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(countdownDefaults)
});
