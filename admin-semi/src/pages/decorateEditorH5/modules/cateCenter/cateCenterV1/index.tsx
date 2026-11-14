/**
 * 分类中心(样式1) 模块（契约依据：uniapp-react components/modules/cate-center/cate-center-v1/config.ts 的 cateCenterV1Defaults，
 * 该文件注明与后台 mobileCateCenter/cateCenterV1 Edit.vue defaultModule 保持一致；入口在 pages/productCate/index.tsx，pageType=4）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

export const cateCenterV1Defaults = {
    searchPlaceholder: "点击搜索商品信息",
    showCategoryName: true,
    showRecommend: true,
    recommendText: "推荐",
    // 颜色一律留空：空串表示跟随主题色 / 原默认色（与渲染端 config 注释一致）
    themeColor: "",
    sideBgColor: "",
    sideTextColor: "",
    sideActiveTextColor: "",
    sideActiveBgColor: "",
    groupTitleColor: "",
    categoryNameColor: ""
};

registerModule({
    type: "cateCenterV1",
    label: "分类中心(样式1)",
    icon: '<i class="h5-mod-ico">🗃️</i>',
    url: "mobileCateCenter",
    platform: "mobile",
    order: 100,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(cateCenterV1Defaults)
});
