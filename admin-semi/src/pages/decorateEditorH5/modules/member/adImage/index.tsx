/**
 * 会员广告图 模块（契约依据：uniapp-react pages/member/components/decorate/adImage/index.tsx）
 * 渲染端读取：pic{picUrl,picThumb}、picLink{link}、frame；无图时渲染端整块不渲染。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const adImageDefaults = {
    pic: { picUrl: "https://oss.tigshop.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picThumb: "" },
    picLink: { link: "", title: "" },
    frame: { ...defaultFrame, backgroundColor: "", boxRadius: 12, innerPadding: 0, boxPadding: 5, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "adImage",
    label: "会员广告图",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 87,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(adImageDefaults)
});
