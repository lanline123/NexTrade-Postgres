/**
 * 订单入口 模块（契约依据：uniapp-react pages/member/components/decorate/orderEntry/index.tsx 实际读取字段）
 * 渲染端读取：title、showTitle、showAllOrders、layoutStyle(1/2)、entryList[{picUrl,picThumb,picTitle,picLink.link}]、
 * badgeBgColor/badgeTextColor、frame。角标数量来自 user store，editor 不配置。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const entry = (picTitle: string) => ({ picUrl: "", picThumb: "", picTitle, picLink: { link: "", title: "" } });

export const orderEntryDefaults = {
    title: "我的订单",
    showTitle: 1,
    showAllOrders: 1,
    layoutStyle: 1,
    entryList: [entry("待付款"), entry("待发货"), entry("待收货"), entry("待评价"), entry("退款/售后")],
    badgeBgColor: "",
    badgeTextColor: "",
    frame: { ...defaultFrame, backgroundColor: "#ffffff", boxRadius: 12, innerPadding: 16, boxPadding: 5, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "orderEntry",
    label: "订单入口",
    icon: '<i class="h5-mod-ico">📦</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 83,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(orderEntryDefaults)
});
