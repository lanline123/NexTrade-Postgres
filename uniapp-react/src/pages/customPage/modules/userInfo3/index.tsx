import UserInfo3 from "@/pages/member/components/decorate/userInfo3";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「用户信息3」（type=userInfo3）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/userInfo3）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
* immersive 由分发器按会员页口径下发：pageModule.immersive 为真且本模块是
* 首个可见模块时才为 true（与 MemberDecorate 的传参一致）。
*/
export default function UserInfo3Module({ module, immersive}: DecorateModuleProps) {
return <UserInfo3 data={module?? {}} immersive={!!immersive} />;
}
