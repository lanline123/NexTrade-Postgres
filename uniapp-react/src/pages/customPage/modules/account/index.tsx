import MemberAccount from "@/pages/member/components/decorate/account";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「账号」（type=account）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/account）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
*/
export default function AccountModule({ module}: DecorateModuleProps) {
return <MemberAccount data={module?? {}} />;
}
