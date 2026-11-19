import MyServiceV2 from "@/pages/member/components/decorate/myServiceV2";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「我的服务」（type=myService）
* 渲染器本体 = 会员页同一组件：MemberDecorate 对 myService 的映射就是
* myServiceV2（layoutStyle + serviceList 口径）；旧 myService 组件只承接
* 历史 menus 兜底数据（数组形状），不是装修模块的数据形状，不在此接入。
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
*/
export default function MyServiceModule({ module}: DecorateModuleProps) {
return <MyServiceV2 data={module?? {}} />;
}
