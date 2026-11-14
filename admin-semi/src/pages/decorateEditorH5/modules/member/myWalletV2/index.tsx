/**
 * 我的钱包(样式2) 模块（契约依据：uniapp-react pages/member/components/decorate/myWalletV2/index.tsx）
 * 渲染端读取：items[{key,label,icon{picUrl}}]（key 为固定枚举 balance/coupon/collect/history，空时用渲染端默认四项）、frame。
 * 各格数值来自 user store / orderNum，editor 不配置。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const myWalletV2Defaults = {
    items: [
        { key: "balance", label: "余额", icon: { picUrl: "", picThumb: "" } },
        { key: "coupon", label: "优惠券", icon: { picUrl: "", picThumb: "" } },
        { key: "collect", label: "商品收藏", icon: { picUrl: "", picThumb: "" } },
        { key: "history", label: "浏览记录", icon: { picUrl: "", picThumb: "" } }
    ],
    frame: { ...defaultFrame, backgroundColor: "#ffffff", boxRadius: 12, innerPadding: 12, boxPadding: 5, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "myWalletV2",
    label: "我的钱包(样式2)",
    icon: '<i class="h5-mod-ico">👛</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 86,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(myWalletV2Defaults)
});
