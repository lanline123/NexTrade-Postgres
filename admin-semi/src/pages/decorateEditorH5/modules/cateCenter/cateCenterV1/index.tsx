/**
 * 分类中心V1 模块（契约自 view/admin editor/modules/mobileCateCenter/cateCenterV1/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobileCateCenter/cateCenterV1/index.ts cateCenterV1Defaults 逐字段一致（契约） */
export const cateCenterV1Defaults = {
    searchPlaceholder: "点击搜索商品信息",
    showCategoryName: true,
    showRecommend: true,
    recommendText: "推荐",
    // 颜色一律留空：空串表示跟随主题色 / 原默认色，保证老数据视觉零变化
    sideBgColor: "",
    sideTextColor: "",
    sideActiveTextColor: "",
    sideActiveBgColor: "",
    groupTitleColor: "",
    categoryNameColor: "",
    frame: { ...cloneDeep(defaultFrame), backgroundColor: "", boxPaddingTop: 0, boxPaddingBottom: 0, boxPadding: 0 }
};

registerModule({
    type: "cateCenterV1",
    label: "分类中心V1",
    icon: '<i class="h5-mod-ico">🗃️</i>',
    url: "mobileCateCenter",
    platform: "mobile",
    order: 1,
    exclusive: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(cateCenterV1Defaults)
});
