/**
 * 用户信息 模块（契约自 view/admin editor/modules/mobileMember/userInfo/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const defaultSignInIcon = "https://oss.tigshop.com/img/gallery/202605/1779691601SIdmgIcby2Mii610eQ.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";
export const defaultSettingsIcon = "https://oss.tigshop.com/img/gallery/202605/1779679345kBAM5aU9K9EuPRQFfo.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";
export const defaultGradient = { colorA: "#fcf6fc", colorB: "#f3f3fe" };

/** 与旧 mobileMember/userInfo/View.vue 的 defaultModule 逐字段一致（契约） */
export const userInfoDefaults = {
    signIn: { show: true, icon: { picUrl: defaultSignInIcon, picThumb: defaultSignInIcon }, title: "签到有礼" },
    settings: { show: true, icon: { picUrl: defaultSettingsIcon, picThumb: defaultSettingsIcon }, title: "设置" },
    gradient: { ...defaultGradient },
    frame: { ...cloneDeep(defaultFrame), backgroundColor: "", boxPaddingTop: 0, boxPaddingBottom: 0, boxPadding: 0, innerPadding: 16 }
};

registerModule({
    type: "userInfo",
    label: "用户信息",
    icon: '<i class="h5-mod-ico">👤</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 1,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(userInfoDefaults)
});
