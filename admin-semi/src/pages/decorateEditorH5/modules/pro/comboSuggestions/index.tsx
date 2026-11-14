/**
 * 组合建议 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/comboSuggestions）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle, defaultProducts } from "../../../contract/sharedDefaults";

export const comboSuggestionsDefaults = {
    comboGroups: [
        { title: "超值搭配", subTitle: "一起买更划算", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", products: cloneDeep(defaultProducts) },
        { title: "人气组合", subTitle: "大家都在买", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", products: cloneDeep(defaultProducts) }
    ],
    products: cloneDeep(defaultProducts),
    autoPlay: 0,
    interval: 3000,
    backgroundGradientType: "leftRight",
    gradientColorA: "#FF7A45",
    gradientColorB: "#FF3630",
    bannerDotColor: "#ffffff",
    bannerTittleColor: "#ffffff",
    groupRadius: 8,
    groupShadow: 1,
    groupTitleColor: "#333333",
    imageRadius: 8,
    regularPriceBgColor: "#f5f5f5",
    regularPriceColor: "#999999",
    regularPriceRadius: 4,
    regularPriceSize: 12,
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "comboSuggestions",
    label: "组合建议",
    icon: '<i class="h5-mod-ico">💡</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 48,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(comboSuggestionsDefaults)
});
