import OrderEntryV2 from "@/pages/member/components/decorate/orderEntryV2";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「我的订单2」（type=orderEntryV2）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/orderEntryV2）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
*/
export default function OrderEntryV2Module({ module}: DecorateModuleProps) {
return <OrderEntryV2 data={module?? {}} />;
}
