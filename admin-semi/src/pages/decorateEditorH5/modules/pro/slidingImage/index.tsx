/**
 * 图片滑动 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/slidingImage）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../../../contract/sharedDefaults";

export const slidingImageDefaults = {
    picList: [
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "", title: "图片标题一", desc: "图片描述", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "", title: "图片标题二", desc: "图片描述", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "", title: "图片标题三", desc: "图片描述", picLink: { link: "", title: "" } }
    ],
    autoPlay: 0,
    interval: 3000,
    backgroundGradientType: "leftRight",
    descColor: "#999999",
    picFillType: "cover",
    picMargin: 8,
    picPadding: 0,
    picRadius: 8,
    picReducePercent: 10,
    picWidth: 80,
    showDesc: 1,
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "slidingImage",
    label: "图片滑动",
    icon: '<i class="h5-mod-ico">🎞️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 66,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(slidingImageDefaults)
});
