/**
 * 我的钱包2 模块（契约自 view/admin editor/modules/mobileMember/myWalletV2/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const WALLET_V2_ICONS: Record<string, string> = {
    balance: "https://oss.tigshop.com/official/img/pc/202605/177993337419dbsyRWczzneL4mhf.png",
    coupon: "https://oss.tigshop.com/official/img/pc/202605/17799333737quaCAwzzMe1XpmYHd.png",
    collect: "https://oss.tigshop.com/official/img/pc/202605/1779933373JYzFsV1Ky8YZ32BzDg.png",
    history: "https://oss.tigshop.com/official/img/pc/202605/1779933373CFhQRjCp2OMvTY1vjQ.png"
};

const item = (key: string, label: string) => ({ key, label, icon: { picUrl: WALLET_V2_ICONS[key], picThumb: WALLET_V2_ICONS[key] } });

/** 与旧 mobileMember/myWalletV2/View.vue 的 defaultModule 逐字段一致（契约） */
export const myWalletV2Defaults = {
    items: [item("balance", "余额"), item("coupon", "优惠券"), item("collect", "商品收藏"), item("history", "浏览记录")],
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        boxRadius: 12,
        innerPadding: 12,
        boxPadding: 10,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "myWalletV2",
    label: "我的钱包2",
    icon: '<i class="h5-mod-ico">👛</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 7,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(myWalletV2Defaults)
});
