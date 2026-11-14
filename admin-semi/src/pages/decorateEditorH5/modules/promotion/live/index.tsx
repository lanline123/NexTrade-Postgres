/**
 * 直播 模块（默认值字段对齐 uniapp-react 渲染端 customPage/modules/live/index.tsx 读取的 module 字段）
 *
 * 渲染端读取：displayMode（"list"|"bigImage"）、sourceType（"auto"|"manual"，渲染端
 * 当前仅消费 selectedLiveList）、autoLiveNum、selectedLiveList（直播间数组，字段见
 * LiveRoomState：roomId/name/anchorName/feedsImg/liveStatus/productData 等）、titleText、
 * frame。渲染端 frame 缺省兜底为 innerPadding 12 / boxRadius 8，此处默认值按此对齐。
 * 注：渲染端仅微信小程序端展示（直播插件为微信专属）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与渲染端 live/index.tsx 读取字段逐项对应 */
export const liveDefaults = {
    titleText: "",
    displayMode: "list",
    sourceType: "auto",
    autoLiveNum: 6,
    selectedLiveList: [] as Array<Record<string, any>>,
    frame: { ...defaultFrame, innerPadding: 12, boxRadius: 8 }
};

registerModule({
    type: "live",
    label: "直播",
    icon: '<i class="h5-mod-ico">📺</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 32,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(liveDefaults)
});
