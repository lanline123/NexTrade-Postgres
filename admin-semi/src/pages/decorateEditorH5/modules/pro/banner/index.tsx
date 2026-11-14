/**
 * 轮播Banner 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/banner）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle } from "../../../contract/sharedDefaults";

export const bannerDefaults = {
    bannerContent: {
        picList: [
            { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "", picLink: { link: "", title: "" }, gradientColorA: "#FF7A45", gradientColorB: "#FF3630" },
    { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "", picLink: { link: "", title: "" }, gradientColorA: "#FF7A45", gradientColorB: "#FF3630" }
        ],
        picRadius: 8,
        picPageType: 1,
        height: 180,
        gradientType: "leftRight",
        backgroundRadius: 0
    },
    bannerStyle: {
        effectType: "default",
        searchBackgroundColor: "#ffffff",
        searchFontColor: "#999999",
        searchIconColor: "#999999",
        searchHeight: 34,
        searchRadius: 20,
        searchIconSize: 20,
        textAlignment: "left",
        navFontSize: 12,
        navFontColor: "#333333",
        navBackgroundColor: "",
        navFontBold: 400,
        navLabelMargin: 4
    },
    searchContent: {
        showSearch: 1,
        searchText: "搜索商品",
        menuIcon: 0,
        menuLink: { link: "", title: "" },
        signIcon: 0,
        signLink: { link: "", title: "" }
    },
    navContent: { showNav: 0, wordList: [] },
    logoPic: { picUrl: "", picThumb: "" },
    logoHeight: 32,
    swiperPageColor: "",
    moduleStyle: defaultModuleStyle
};

registerModule({
    type: "banner",
    label: "轮播Banner",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 40,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(bannerDefaults)
});
