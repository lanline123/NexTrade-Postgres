/**
 * 首焦轮播A2 模块（契约自 view/admin editor/modules/mobilePro/bannerA2/index.ts 逐字段移植）
 *
 * 毛玻璃背景大图 + 搜索框悬浮 + 轮播 + 指示器（跨境版首焦样式）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle } from "../../../contract/sharedDefaults";
import { isStore } from "../../../contract/version";

const picList = [
    {
        picId: 1414,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731044101tXK31aimefqoUsJSrM.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731301937REbUnsSmPehWiBMaFd.webp",
        picName: "FjfhKDiUzqW_lqfJqncKt3QJplx4"
    },
    {
        picId: 1415,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731044101j8COoqtfDJCWrSts6b.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731301941uaSjIShOBNL2VBSr0y.webp",
        picName: "Fv5iP2IHByVFUlGrLkXtPkClndla"
    },
    {
        picId: 1413,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731044101D4owQpnWiu0DcTqmKO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731301927a0N31LV7yy1i6Zlk4T.webp",
        picName: "FhHfxuZP5Z5P_Hc7SHBzvm6RwHud"
    }
];

/** 与旧 mobilePro/bannerA2/index.ts 逐字段一致（契约） */
export const bannerA2Defaults = {
    bannerContent: {
        picList,
        picRadius: 0,
        picPageType: 1, // 指示器样式 1/2
        swiperPageColor: "",
        indicatorColor: ""
    },
    searchContent: {
        showSearch: 1,
        searchText: "搜索商品"
    },
    bannerStyle: {
        effectType: "ceiling", // ceiling 滚动至顶部固定 / default 正常
        searchBackgroundColor: "#ffffff",
        searchFontColor: "#999999",
        searchIconColor: "#999999",
        searchText: "搜索",
        searchHeight: 34,
        searchRadius: 20
    },
    moduleStyle: defaultModuleStyle
};

registerModule({
    type: "bannerA2",
    label: "首焦轮播A2",
    icon: '<i class="h5-mod-ico">🌄</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 68,
    // 原版 enabled: () => isPro() && isStore()，映射为 isStore()（我方即 pro 版，同 storeNav 口径）
    enabled: () => isStore(),
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(bannerA2Defaults)
});
