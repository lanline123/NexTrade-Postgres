/**
 * 我的订单2 模块（契约自 view/admin editor/modules/mobileMember/orderEntryV2/index.ts 逐字段移植）
 *
 * 原版注册带 enabled: () => isOverseas()（仅海外版可见）——按原版口径保留（我方 isOverseas 读 VITE_IS_OVERSEAS）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";
import { isOverseas } from "../../../contract/version";

const JPIC = "?x-oss-process=image/resize,m_pad,h_200,h_200";
const j = (name: string) => `https://oss.tigshop.com/img/gallery/202605/${name}${JPIC}`;

/** 与旧 mobileMember/orderEntryV2/View.vue 的 defaultModule 逐字段一致（契约） */
export const orderEntryV2Defaults = {
    title: "订单",
    cardList: [
        { picUrl: j("1779688569TRT9GOOm6YGq6RHtPG.jpeg"), picThumb: j("1779688569TRT9GOOm6YGq6RHtPG.jpeg"), picTitle: "待付款" },
        { picUrl: j("1779688569I3uI9owxG28WqM4kZq.jpeg"), picThumb: j("1779688569I3uI9owxG28WqM4kZq.jpeg"), picTitle: "进行中" },
        { picUrl: j("1779688569bsPmvK2X8p96DhrC8f.jpeg"), picThumb: j("1779688569bsPmvK2X8p96DhrC8f.jpeg"), picTitle: "已完成" }
    ],
    listItems: [
        { picUrl: j("1779688569mV8f2OAjjvfB6GURqx.jpeg"), picThumb: j("1779688569mV8f2OAjjvfB6GURqx.jpeg"), picTitle: "评价" },
        { picUrl: j("1779688569FJBk502CI7W1gOfH0l.jpeg"), picThumb: j("1779688569FJBk502CI7W1gOfH0l.jpeg"), picTitle: "售后" }
    ],
    cardBackgroundColor: "#f6f7fb",
    badgeBgColor: "#ff4d4f",
    badgeTextColor: "#ffffff",
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        boxRadius: 12,
        innerPadding: 16,
        boxPadding: 0,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "orderEntryV2",
    label: "我的订单2",
    icon: '<i class="h5-mod-ico">📦</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 5,
    enabled: () => isOverseas(),
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(orderEntryV2Defaults)
});
