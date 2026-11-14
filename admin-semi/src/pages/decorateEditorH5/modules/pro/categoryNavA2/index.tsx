/**
 * 分类导航A2 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/categoryNavA2）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultAnimation, defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../../../contract/sharedDefaults";

export const categoryNavA2Defaults = {
    listStyleType: "list",
    categoryList: [
        { picId: 1377, picThumb: "", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picName: "", picTitle: "当日速达", picDesc: "极速送达" },
    { picId: 1384, picThumb: "", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picName: "", picTitle: "休闲零食", picDesc: "美味零食" },
    { picId: 1391, picThumb: "", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picName: "", picTitle: "日常用品", picDesc: "居家必备" },
    { picId: 1398, picThumb: "", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957950bBFeVKuTtCEWKllS.png", picName: "", picTitle: "数码家电", picDesc: "智能生活" },
    { picId: 1405, picThumb: "", picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957951pDjgKRAwAEZTKNwv.png", picName: "", picTitle: "美妆护肤", picDesc: "美丽秘诀" }
    ],
    rowNum: 5,
    rowPicPercent: 90,
    indicatorBar: 1,
    barColor: "#e5e5e5",
    barColorOn: "#333333",
    barWidth: 20,
    barWidthOn: 30,
    barHeight: 3,
    animation: defaultAnimation,
    picFillType: "cover",
    picPercent: 90,
    radius: 200,
    categoryPadding: 10,
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryNavA2",
    label: "分类导航(样式2)",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 44,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryNavA2Defaults)
});
