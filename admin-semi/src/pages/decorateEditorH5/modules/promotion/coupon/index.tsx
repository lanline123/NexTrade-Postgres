/**
 * 优惠券 模块（默认值字段对齐 uniapp-react 渲染端 customPage/modules/coupon/index.tsx 读取的 module 字段）
 *
 * 渲染端读取：frame（formatFrame）、colorStyle、showTitle、title（字符串标题）、desc、
 * backgroundColor、itemBackgroundColor、boxPadding、boxMargin、boxPaddingTop、boxPaddingBottom、
 * boxRadius、itemPadding；券列表由渲染端接口 getHomeCoupon 拉取，不存模块数据。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与渲染端 coupon/index.tsx 读取字段逐项对应 */
export const couponDefaults = {
    colorStyle: 1,
    showTitle: 1,
    title: "领券中心",
    desc: "",
    backgroundColor: "",
    itemBackgroundColor: "",
    boxPadding: 10,
    boxMargin: 0,
    boxPaddingTop: 5,
    boxPaddingBottom: 5,
    boxRadius: 0,
    itemPadding: 0,
    frame: defaultFrame
};

registerModule({
    type: "coupon",
    label: "优惠券",
    icon: '<i class="h5-mod-ico">🎫</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 30,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(couponDefaults)
});
