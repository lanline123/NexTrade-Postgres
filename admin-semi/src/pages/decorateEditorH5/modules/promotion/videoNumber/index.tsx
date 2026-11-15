/**
 * 视频号 模块（契约自 view/admin editor/modules/mobilePromotion/videoNumber/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobilePromotion/videoNumber/View.vue 的 defaultModule 逐字段一致（契约；liveTitle/titleColor 取旧 View 局部常量字面量） */
export const videoNumberDefaults = {
    titleText: "视频号",
    contentType: "video",
    videoNumberId: "",
    videoId: "",
    liveTitle: "",
    titleColorType: "default",
    titleColor: "#2A2A2A",
    productImages: [] as Record<string, any>[],
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
    type: "videoNumber",
    label: "视频号",
    icon: '<i class="h5-mod-ico">📹</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 2,
    toolbarGroup: "live",
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(videoNumberDefaults)
});
