/**
 * 拼团 模块（契约自 view/admin editor/modules/mobilePromotion/groupon/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame, defaultTitle } from "../../../contract/sharedDefaults";

/** 与旧 mobilePromotion/groupon/View.vue 的 defaultModule 逐字段一致（契约；grouponActivityItem 默认为 undefined，JSON 化后即无此键） */
export const grouponDefaults = {
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
    // 边距由顶层 boxPadding* 控制，frame 内边距置 0，避免与外层叠成双倍空隙
    frame: {
        ...cloneDeep(defaultFrame),
        boxPadding: 0,
        boxPaddingTop: 0,
        boxPaddingBottom: 0
    },
    title: cloneDeep(defaultTitle),
    groupType: 1
};

registerModule({
    type: "groupon",
    label: "拼团",
    icon: '<i class="h5-mod-ico">👥</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 4,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(grouponDefaults)
});
