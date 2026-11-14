/**
 * 分类中心(样式2) 模块（契约依据：uniapp-react components/modules/cate-center/cate-center-v2/config.ts 的 cateCenterV2Defaults，
 * 该文件注明与后台 mobileCateCenter/cateCenterV2 Edit.vue defaultModule 保持一致；入口在 pages/productCate/index.tsx，pageType=4）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

export const cateCenterV2Defaults = {
    searchPlaceholder: "搜索商品",
    styleType: 2,
    subTabStyle: 1,
    textAlign: 1,
    textWeight: 1,
    goodsNameRow: 2,
    showName: 1,
    buyBtnStyle: 1,
    // 颜色一律留空：空串表示跟随主题色 / 原默认色（与渲染端 config 注释一致）
    themeColor: "",
    sideBgColor: "",
    sideTextColor: "",
    sideActiveTextColor: "",
    sideActiveBgColor: "",
    sideIndicatorColor: "",
    subTabTextColor: "",
    subTabBgColor: "",
    subTabActiveTextColor: "",
    subTabActiveBgColor: "",
    goodsListBgColor: "",
    productTitleColor: "",
    priceColor: "",
    buyBtnColor: ""
};

registerModule({
    type: "cateCenterV2",
    label: "分类中心(样式2)",
    icon: '<i class="h5-mod-ico">🗃️</i>',
    url: "mobileCateCenter",
    platform: "mobile",
    order: 101,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(cateCenterV2Defaults)
});
