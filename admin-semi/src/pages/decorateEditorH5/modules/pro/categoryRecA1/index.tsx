/**
 * 分类推荐A1 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/categoryRecA1）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../../../contract/sharedDefaults";

export const categoryRecA1Defaults = {
    picList: [
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "", title: "热门推荐", desc: "精选好物", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "", title: "新品上市", desc: "抢先体验", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "", title: "限时特惠", desc: "低价直降", picLink: { link: "", title: "" } }
    ],
    picRadius: 8,
    picFillType: "cover",
    subTitle: defaultSubTitle,
    subTitleColor: "#999999",
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryRecA1",
    label: "分类推荐(样式1)",
    icon: '<i class="h5-mod-ico">🗂️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 47,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryRecA1Defaults)
});
