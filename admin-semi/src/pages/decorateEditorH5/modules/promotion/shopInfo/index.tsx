/**
 * 店铺信息 模块（契约自 view/admin editor/modules/mobilePromotion/shopInfo/index.ts 逐字段移植）
 *
 * 原版注册带 shopOnly: true（仅店铺端 adminType=shop 显示）——按原版口径保留。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobilePromotion/shopInfo/View.vue 的 defaultModule 逐字段一致（契约） */
export const shopInfoDefaults = {
    customPic: { picUrl: "", picThumb: "" },
    backgroundDefault: 1,
    style: 1,
    frame: cloneDeep(defaultFrame)
};

registerModule({
    type: "shopInfo",
    label: "店铺信息",
    icon: '<i class="h5-mod-ico">🏪</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 5,
    shopOnly: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(shopInfoDefaults)
});
