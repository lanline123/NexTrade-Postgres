/**
 * 用户信息3 模块（契约自 view/admin editor/modules/mobileMember/userInfo3/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const SIGNIN_ICON = "https://oss.tigshop.com/img/gallery/202605/1779691601SIdmgIcby2Mii610eQ.jpeg?x-oss-process=image/resize,m_pad,h_200,h_200";
export const USERINFO3_SETTINGS_ICON = "https://oss.tigshop.com/official/img/pc/202605/1779872068yYRoxuCDxWgGcOti6b.png";
export const USERINFO3_NOTIFICATION_ICON = "https://oss.tigshop.com/official/img/pc/202605/1779958812O0gl221tWQVIEt7OXD.png";
export const USERINFO3_GROWTH_CARD_BG = "https://oss.tigshop.com/official/img/pc/202606/17808830195Hk80LmOeUpDCCCEKW.png";
const STYLE1_BG = "https://oss.tigshop.com/official/img/pc/202605/1779860190FsB8OM4MrGwgOCvKsA.png";

/** 与旧 mobileMember/userInfo3/View.vue 的 mergeDefaultModule 字面量逐字段一致（契约） */
export const userInfo3Defaults = {
    layoutStyle: 1,
    showEditIcon: true,
    signIn: { show: true, icon: { picUrl: SIGNIN_ICON, picThumb: SIGNIN_ICON }, title: "签到有礼" },
    settings: { show: true, icon: { picUrl: USERINFO3_SETTINGS_ICON, picThumb: USERINFO3_SETTINGS_ICON } },
    gradient: { colorA: "#E0FCED", colorB: "#FDF1F3" },
    bgColor: "#57D45C",
    backgroundImage: { picUrl: STYLE1_BG, picThumb: STYLE1_BG },
    bottomArc: { show: true, height: 24 },
    memberBar: {
        show: true,
        backgroundColor: "",
        borderRadius: 10,
        levelName: "付费会员",
        description: "购买会员，畅享更多专属权益",
        buttonText: "查看",
        buttonLink: { path: "base", label: "权益卡中心", name: "权益卡中心", link: "member/premiumMember/index" },
        waveImage: { picUrl: USERINFO3_GROWTH_CARD_BG, picThumb: USERINFO3_GROWTH_CARD_BG }
    },
    notification: { show: true, icon: { picUrl: USERINFO3_NOTIFICATION_ICON, picThumb: USERINFO3_NOTIFICATION_ICON } },
    stats: {
        show: true,
        items: [
            { key: "collect", label: "商品收藏", show: true },
            { key: "follow", label: "门店关注", show: true },
            { key: "history", label: "浏览记录", show: true }
        ]
    },
    frame: { ...cloneDeep(defaultFrame), backgroundColor: "", boxPaddingTop: 0, boxPaddingBottom: 0, boxPadding: 0, innerPadding: 16 }
};

registerModule({
    type: "userInfo3",
    label: "用户信息3",
    icon: '<i class="h5-mod-ico">👤</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 3,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(userInfo3Defaults)
});
