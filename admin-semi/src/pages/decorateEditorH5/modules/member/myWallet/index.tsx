/**
 * 我的钱包 模块（契约依据：uniapp-react pages/member/components/decorate/myWallet/index.tsx）
 * 渲染端读取：layoutStyle(1/2)、walletIcon{picUrl}、frame。余额/优惠券/积分来自 user store，editor 不配置。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const myWalletDefaults = {
    layoutStyle: 1,
    walletIcon: { picUrl: "", picThumb: "" },
    frame: { ...defaultFrame, backgroundColor: "#ffffff", boxRadius: 12, innerPadding: 16, boxPadding: 5, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "myWallet",
    label: "我的钱包",
    icon: '<i class="h5-mod-ico">👛</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 85,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(myWalletDefaults)
});
