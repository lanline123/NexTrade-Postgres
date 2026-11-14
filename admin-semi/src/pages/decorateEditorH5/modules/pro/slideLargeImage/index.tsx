/**
 * 大图滑动 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/slideLargeImage）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../../../contract/sharedDefaults";

export const slideLargeImageDefaults = {
    picList: [
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "", title: "大图标题一", desc: "大图描述", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "", title: "大图标题二", desc: "大图描述", picLink: { link: "", title: "" } }
    ],
    rowNum: 1,
    picMargin: 8,
    picMarginTop: 0,
    picRadius: 8,
    slidingEffect: "slide",
    slidingSpeed: 3000,
    picTitleDesc: { backgroundColor: "", fontPosition: "left" },
    moduleTitle: { title: "", titlePosition: "left" },
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "slideLargeImage",
    label: "大图滑动",
    icon: '<i class="h5-mod-ico">🌌</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 65,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(slideLargeImageDefaults)
});
