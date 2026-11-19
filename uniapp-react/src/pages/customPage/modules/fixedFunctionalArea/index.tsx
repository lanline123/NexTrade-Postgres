import FixedFunctionalArea from "@/pages/member/components/decorate/fixedFunctionalArea";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「固定功能区」（type=fixedFunctionalArea）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/fixedFunctionalArea）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
* 该组件按会员页真实口径渲染：无账期/授信内容时整块不渲染（返回 null），
* 与线上会员页一致；分发器外层 View 仍占一个槽位，布局序号不位移。
*/
export default function FixedFunctionalAreaModule({ module}: DecorateModuleProps) {
return <FixedFunctionalArea data={module?? {}} />;
}
