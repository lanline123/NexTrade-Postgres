import MyWalletV2 from "@/pages/member/components/decorate/myWalletV2";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「我的钱包2」（type=myWalletV2）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/myWalletV2）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
*/
export default function MyWalletV2Module({ module}: DecorateModuleProps) {
return <MyWalletV2 data={module?? {}} />;
}
