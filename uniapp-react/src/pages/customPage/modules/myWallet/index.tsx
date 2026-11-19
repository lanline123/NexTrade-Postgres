import MyWallet from "@/pages/member/components/decorate/myWallet";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「我的钱包」（type=myWallet）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/myWallet）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
*/
export default function MyWalletModule({ module}: DecorateModuleProps) {
return <MyWallet data={module?? {}} />;
}
