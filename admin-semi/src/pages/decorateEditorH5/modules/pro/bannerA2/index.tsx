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
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731044101tXK31aimefqoUsJSrM.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731301937REbUnsSmPehWiBMaFd.webp",
        picName: "FjfhKDiUzqW_lqfJqncKt3QJplx4"
    },
    {
        picId: 1415,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731044101j8COoqtfDJCWrSts6b.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731301941uaSjIShOBNL2VBSr0y.webp",
        picName: "Fv5iP2IHByVFUlGrLkXtPkClndla"
    },
    {
        picId: 1413,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731044101D4owQpnWiu0DcTqmKO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731301927a0N31LV7yy1i6Zlk4T.webp",
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
    // 原版 enabled: () => isPro() && isStore()：isPro 省略（我方即 pro 版）；isStore 为原版
    // VITE_IS_STORE 版本标志（contract/version 已按原版 env 口径实现，全量版 .env 置 1 恒真）。
    // 原版注册无 shopOnly——2026-11-21 随点单卡片复核一并删除误加项，报告见 storeordercard-audit。
    enabled: () => isStore(),
    // 店铺端亦可见：用户 2026-11-21 裁决「全部的都可见」（原版有·我方改口径，
    // 原版店铺端不见高级组），经 shopVisible 显式豁免，见 storeordercard-audit/REPORT2.md。
    shopVisible: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(bannerA2Defaults)
});
