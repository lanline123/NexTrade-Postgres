/**
 * 热图绘制 模块（契约自 view/admin editor/modules/mobile/imageHotarea/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame, defaultTitle } from "../../../contract/sharedDefaults";

/**
 * 我方新增默认演示图（2026-11-21）：原版 defaults 的 picList 为空，新增模块在
 * 画布上是空块、无从画热区；预置 1 张已迁移演示图（与首焦轮播演示图同源，
 * 实测 200）作画布底图，热区由用户自行绘制。仅新增模块生效，不回填存量数据。
 */
const demoPicList = [
    {
        picId: 1413,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731044101D4owQpnWiu0DcTqmKO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731301927a0N31LV7yy1i6Zlk4T.webp",
        picName: "FhHfxuZP5Z5P_Hc7SHBzvm6RwHud"
    }
];

/** 与 view/admin imageHotareaDefaults 逐字段一致（契约），仅 picList 为我方新增默认演示图 */
export const imageHotareaDefaults = {
    picList: demoPicList,
    imgPadding: 0,
    picRadioStyle: 1,
    frame: defaultFrame,
    title: defaultTitle
};

registerModule({
    type: "imageHotarea",
    label: "热图绘制",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobile",
    platform: "mobile",
    order: 4,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(imageHotareaDefaults)
});
