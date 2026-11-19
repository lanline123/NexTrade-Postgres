/**
 * 我的钱包 模块（契约自 view/admin editor/modules/mobileMember/myWallet/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const WALLET_ICON_URL = "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779873859zO64yVrNvp2y0lUsOz.png";

/** 与旧 mobileMember/myWallet/View.vue 的 defaultModule 逐字段一致（契约） */
export const myWalletDefaults = {
    layoutStyle: 1,
    walletIcon: { picUrl: WALLET_ICON_URL, picThumb: WALLET_ICON_URL },
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        boxRadius: 12,
        innerPadding: 16,
        boxPadding: 5,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "myWallet",
    label: "我的钱包",
    icon: '<i class="h5-mod-ico">👛</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 6,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(myWalletDefaults)
});
