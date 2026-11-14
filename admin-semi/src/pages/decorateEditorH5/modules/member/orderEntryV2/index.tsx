/**
 * 订单入口(样式2) 模块（跨境「我的订单2」；契约依据：uniapp-react pages/member/components/decorate/orderEntryV2/index.tsx）
 * 渲染端读取：title、showTitle、showMore、cardList[{picUrl,picTitle,picLink.link}]、listItems[同]、
 * cardBackgroundColor、badgeBgColor/badgeTextColor、frame（moduleStyle.cardBackgroundColor 为旧兼容回退）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const entry = (picTitle: string) => ({ picUrl: "", picThumb: "", picTitle, picLink: { link: "", title: "" } });

export const orderEntryV2Defaults = {
    title: "订单",
    showTitle: 1,
    showMore: 1,
    cardList: [entry("待付款"), entry("进行中"), entry("已完成")],
    listItems: [entry("待评价"), entry("退款/售后")],
    cardBackgroundColor: "",
    badgeBgColor: "",
    badgeTextColor: "",
    frame: { ...defaultFrame, backgroundColor: "#ffffff", boxRadius: 12, innerPadding: 16, boxPadding: 0, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "orderEntryV2",
    label: "订单入口(样式2)",
    icon: '<i class="h5-mod-ico">📦</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 84,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(orderEntryV2Defaults)
});
