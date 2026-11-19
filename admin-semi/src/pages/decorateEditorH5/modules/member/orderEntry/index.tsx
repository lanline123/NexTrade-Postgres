/**
 * 我的订单 模块（契约自 view/admin editor/modules/mobileMember/orderEntry/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const JPIC = "?x-oss-process=image/resize,m_pad,h_200,h_200";
const j = (name: string) => `https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/${name}${JPIC}`;
const p = (name: string) => `https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/${name}`;

/** 样式一入口（JPEG 图标，原版 Settings.vue style1EntryList） */
export const orderEntryStyle1List = [
    { picUrl: j("1779688296LEk49r7zpXWPuZYHE1.jpeg"), picThumb: j("1779688296LEk49r7zpXWPuZYHE1.jpeg"), picTitle: "待付款" },
    { picUrl: j("1779688296MMBp6rwui5XUCK23J1.jpeg"), picThumb: j("1779688296MMBp6rwui5XUCK23J1.jpeg"), picTitle: "待发货" },
    { picUrl: j("1779688296yHIquz41WhOu77OOOM.jpeg"), picThumb: j("1779688296yHIquz41WhOu77OOOM.jpeg"), picTitle: "待收货" },
    { picUrl: j("1779688297NOaA9Zx6mSws3oqgh6.jpeg"), picThumb: j("1779688297NOaA9Zx6mSws3oqgh6.jpeg"), picTitle: "待评价" },
    { picUrl: j("1779688297PzS7ITTZI1NC5BXiG4.jpeg"), picThumb: j("1779688297PzS7ITTZI1NC5BXiG4.jpeg"), picTitle: "退换/售后" }
];

/** 样式二入口（PNG 图标，原版 Settings.vue style2EntryList；无「待评价」，末项「我的订单」） */
export const orderEntryStyle2List = [
    { picUrl: p("1779948168bVpgRKHWQeyik4YB5i.png"), picThumb: p("1779948168bVpgRKHWQeyik4YB5i.png"), picTitle: "待付款" },
    { picUrl: p("1779948167OzuyBnYUTGf6sMQcS4.png"), picThumb: p("1779948167OzuyBnYUTGf6sMQcS4.png"), picTitle: "待发货" },
    { picUrl: p("1779948168Ool4xEFRoCZqQ5eTDq.png"), picThumb: p("1779948168Ool4xEFRoCZqQ5eTDq.png"), picTitle: "待收货" },
    { picUrl: p("1779948168iIVh0NpHDDSf41hFPN.png"), picThumb: p("1779948168iIVh0NpHDDSf41hFPN.png"), picTitle: "退换/售后" },
    { picUrl: p("1779948169KmcNUMPH14twh8qBdD.png"), picThumb: p("1779948169KmcNUMPH14twh8qBdD.png"), picTitle: "我的订单" }
];

/** 角标默认背景（原版 getDefaultBadgeBgColor：样式一蓝、样式二红） */
export const orderEntryBadgeBg = (layoutStyle?: number) => (layoutStyle === 1 ? "#4d9be4" : "#ff4d4f");

/** 与旧 mobileMember/orderEntry/View.vue 的 defaultModule 逐字段一致（契约） */
export const orderEntryDefaults = {
    title: "我的订单",
    layoutStyle: 1,
    entryList: orderEntryStyle1List.map((i) => ({ ...i })),
    badgeBgColor: "#4d9be4",
    badgeTextColor: "#ffffff",
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        boxRadius: 12,
        innerPadding: 16,
        boxPadding: 10,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "orderEntry",
    label: "我的订单",
    icon: '<i class="h5-mod-ico">📦</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 4,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(orderEntryDefaults)
});
