/**
 * 视频号 模块（默认值字段对齐 uniapp-react 渲染端 customPage/modules/videoNumber/index.tsx 读取的 module 字段）
 *
 * 渲染端读取：contentType（"video"|"live"）、videoNumberId（视频号 ID）、videoId
 * （视频/直播 feedId）、liveTitle、titleColorType（"default"|"theme"|"custom"）、
 * titleColor、productImages（{picUrl,picThumb} 数组，最多展示 3 张）、frame。
 * 渲染端 frame 缺省兜底为 innerPadding 12 / boxRadius 8，此处默认值按此对齐。
 * 注：渲染端仅微信小程序端展示（channel-video/channel-live 为微信专属组件）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与渲染端 videoNumber/index.tsx 读取字段逐项对应 */
export const videoNumberDefaults = {
    contentType: "video",
    videoNumberId: "",
    videoId: "",
    liveTitle: "",
    titleColorType: "default",
    titleColor: "",
    productImages: [] as Array<Record<string, any>>,
    frame: { ...defaultFrame, innerPadding: 12, boxRadius: 8 }
};

registerModule({
    type: "videoNumber",
    label: "视频号",
    icon: '<i class="h5-mod-ico">🎬</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 36,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(videoNumberDefaults)
});
