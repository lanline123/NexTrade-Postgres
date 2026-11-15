/**
 * 秒杀 模块（契约自 view/admin editor/modules/mobilePromotion/seckill/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame, defaultTitle } from "../../../contract/sharedDefaults";

/** 与旧 mobilePromotion/seckill/View.vue 的 defaultModule 逐字段一致（契约） */
export const seckillDefaults = {
    style: 1,
    goodsStyle: 1,
    goodsRadioStyle: 1,
    textAlign: 1,
    textWeight: 1,
    goodsNameRow: 2,
    goodsNamePadding: 1,
    showName: 1,
    showBrief: 1,
    showPrice: 1,
    goodsPadding: 5,
    buyBtnStyle: 1,
    backgroundColor: "",
    boxRadius: 0,
    innerPadding: 0,
    boxPadding: 10,
    boxPaddingTop: 5,
    boxPaddingBottom: 5,
    frame: cloneDeep(defaultFrame),
    title: cloneDeep(defaultTitle)
};

registerModule({
    type: "seckill",
    label: "秒杀",
    icon: '<i class="h5-mod-ico">⚡</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 2,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(seckillDefaults)
});
