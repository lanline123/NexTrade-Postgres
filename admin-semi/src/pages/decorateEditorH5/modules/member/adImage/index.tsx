/**
 * 广告图 模块（契约自 view/admin editor/modules/mobileMember/adImage/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const DEFAULT_PIC_URL = "https://oss.tigshop.com/img/gallery/202605/1779689214Nbzv6Iw3ldEVFucDjc.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";

/** 与旧 mobileMember/adImage/View.vue 的 defaultModule 逐字段一致（契约） */
export const adImageDefaults = {
    pic: { picUrl: DEFAULT_PIC_URL, picThumb: DEFAULT_PIC_URL },
    picLink: { link: "", name: "" },
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "",
        boxRadius: 12,
        innerPadding: 0,
        boxPadding: 5,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "adImage",
    label: "广告图",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 8,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(adImageDefaults)
});
