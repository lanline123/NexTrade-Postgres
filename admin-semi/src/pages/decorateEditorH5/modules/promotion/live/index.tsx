/**
 * 小程序直播 模块（契约自 view/admin editor/modules/mobilePromotion/live/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobilePromotion/live/View.vue 的 defaultModule 逐字段一致（契约） */
export const liveDefaults = {
    titleText: "小程序直播",
    displayMode: "list",
    sourceType: "auto",
    autoLiveNum: 10,
    selectedLiveList: [] as Record<string, any>[],
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        innerPadding: 12,
        boxRadius: 8,
        boxPadding: 10,
        boxPaddingTop: 5,
        boxPaddingBottom: 5
    }
};

registerModule({
    type: "live",
    label: "小程序直播",
    icon: '<i class="h5-mod-ico">📺</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 1,
    toolbarGroup: "live",
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(liveDefaults)
});
