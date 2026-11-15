/**
 * 首焦轮播 模块（契约自 view/admin editor/modules/mobilePro/banner/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle } from "../../../contract/sharedDefaults";

const navList = [
    {
        path: "",
        label: "",
        name: "",
        data: {
            name: "",
            path: ""
        },
        picTitle: "新品",
        picLink: {}
    },
    {
        path: "",
        label: "",
        name: "",
        data: {
            name: "",
            path: ""
        },
        picTitle: "百货"
    },
    {
        path: "",
        label: "",
        name: "",
        data: {
            name: "",
            path: ""
        },
        picTitle: "零食"
    },
    {
        path: "",
        label: "",
        name: "",
        data: {
            name: "",
            path: ""
        },
        picTitle: "折扣"
    },
    {
        path: "",
        label: "",
        name: "",
        data: {
            name: "",
            path: ""
        },
        picTitle: "猜你喜欢"
    }
];
const picList = [
    {
        picId: 1414,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731044101tXK31aimefqoUsJSrM.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731301937REbUnsSmPehWiBMaFd.webp",
        picName: "FjfhKDiUzqW_lqfJqncKt3QJplx4",
        gradientColorA: "#62a05c",
        gradientColorB: "#72b76c"
    },
    {
        picId: 1415,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731044101j8COoqtfDJCWrSts6b.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731301941uaSjIShOBNL2VBSr0y.webp",
        picName: "Fv5iP2IHByVFUlGrLkXtPkClndla",
        gradientColorA: "#62a05c",
        gradientColorB: "#72b76c"
    },
    {
        picId: 1413,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731044101D4owQpnWiu0DcTqmKO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731301927a0N31LV7yy1i6Zlk4T.webp",
        picName: "FhHfxuZP5Z5P_Hc7SHBzvm6RwHud",
        gradientColorA: "#62a05c",
        gradientColorB: "#72b76c"
    }
];
const logo = {
    picUrl: "http://oss.tigshop.com/img/gallery/202409/1727056331nhxqtK8Mm5u8MXokoI.png",
    picThumb: "https://oss.tigshop.com/img/gallery/202409/1727056331nhxqtK8Mm5u8MXokoI.png?x-oss-process=image/resize,m_pad,h_200,h_200",
    picId: 1221,
    picName: "logoWhite"
};

/** 与旧 mobilePro/banner/index.ts bannerDefaults 逐字段一致（契约） */
export const bannerDefaults = {
    logoPic: logo,
    logoHeight: 100,
    bannerContent: {
        picList,
        picRadius: 6,
        height: 200,
        backgroundRadius: 50,
        gradientType: "upDown",
        picPageType: 1,
        swiperPageColor: ""
    },
    searchContent: {
        showSearch: 1,
        searchText: "搜索商品",
        menuIcon: 1,
        menuLink: "",
        menuPic: { picUrl: "", picThumb: "" },
        signIcon: 1,
        signLink: "",
        signPic: { picUrl: "", picThumb: "" }
    },
    navContent: {
        showNav: 1,
        navList
    },
    bannerStyle: {
        effectType: "ceiling",
        searchBackgroundColor: "#ffffff",
        searchFontColor: "#999999",
        showSearchBtn: 1,
        searchText: "搜索",
        searchHeight: 34,
        searchRadius: 20,
        searchIconSize: 24,
        navPrefix: "热搜:",
        navFontColor: "#ffffff",
        navBackgroundColor: "rgba(255, 255, 255, 0.3)",
        navPrefixColor: "#ffffff",
        textAlignment: "center",
        navFontBold: 400,
        navLabelMargin: 5,
        navFontSize: 12
    },
    moduleStyle: defaultModuleStyle
};

registerModule({
    type: "banner",
    label: "首焦轮播",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 16,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(bannerDefaults)
});
