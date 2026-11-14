/**
 * 店铺导航 模块（mobilePro）
 *
 * 注意：storeNav 在 uniapp-react 主 registry 与 shopModules 店铺白名单中均无渲染入口（已核实），
 * 故 registerModule 加 visible:false 不进工具栏——仅保证存量数据可打开/预览，
 * 待渲染端补入口后再放开。契约定义与 Preview/Settings 照常做全。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../../../contract/sharedDefaults";

export const storeNavDefaults = {
    picList: [
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picThumb: "", picTitle: "店铺首页", title: "店铺首页", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "", picTitle: "全部商品", title: "全部商品", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957949mTiXqnQpGXzYHnNV.png", picThumb: "", picTitle: "店铺分类", title: "店铺分类", picLink: { link: "", title: "" } },
        { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957950bBFeVKuTtCEWKllS.png", picThumb: "", picTitle: "联系客服", title: "联系客服", picLink: { link: "", title: "" } }
    ],
    rowNum: 4,
    picFillType: "cover",
    picPercent: 80,
    radius: 200,
    categoryPadding: 8,
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "storeNav",
    label: "店铺导航",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 68,
    visible: false,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(storeNavDefaults)
});
