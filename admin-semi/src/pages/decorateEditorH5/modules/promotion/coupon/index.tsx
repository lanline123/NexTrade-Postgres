/**
 * 优惠券 模块（契约自 view/admin editor/modules/mobilePromotion/coupon/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobilePromotion/coupon/View.vue 的 defaultModule 逐字段一致（契约） */
export const couponDefaults = {
    showTitle: 0,
    title: "每日领券",
    desc: "天天优惠领不停",
    colorStyle: 0,
    itemBackgroundColor: "#ffffff",
    backgroundColor: "#ffffff",
    itemPadding: 10,
    boxRadius: 8,
    boxPadding: 10,
    boxMargin: 0,
    boxPaddingTop: 5,
    boxPaddingBottom: 5,
    frame: cloneDeep(defaultFrame)
};

registerModule({
    type: "coupon",
    label: "优惠券",
    icon: '<i class="h5-mod-ico">🎟️</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 1,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(couponDefaults)
});
