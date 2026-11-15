/**
 * 秒杀2 模块（契约自 view/admin editor/modules/mobilePromotion/seckill2/index.ts 逐字段移植）
 *
 * 原版注册带 enabled: () => isOverseas()（仅海外版可见）——按原版口径保留。defaults 无 frame 字段（契约）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultTitle } from "../../../contract/sharedDefaults";
import { isOverseas } from "../../../contract/version";

/** 与旧 mobilePromotion/seckill2/View.vue 的 defaultModule 逐字段一致（契约，无 frame 字段） */
export const seckill2Defaults = {
    style: 1,
    goodsNamePadding: 1,
    showName: 0,
    showPrice: 1,
    goodsPadding: 5,
    imageBackgroundColor: "",
    backgroundColor: "",
    boxRadius: 0,
    innerPadding: 0,
    boxPadding: 10,
    boxPaddingTop: 5,
    boxPaddingBottom: 5,
    title: cloneDeep(defaultTitle)
};

registerModule({
    type: "seckill2",
    label: "秒杀2",
    icon: '<i class="h5-mod-ico">⚡</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 3,
    enabled: () => isOverseas(),
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(seckill2Defaults)
});
