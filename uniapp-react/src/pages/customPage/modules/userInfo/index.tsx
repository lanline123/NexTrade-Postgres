import UserInfo from "@/pages/member/components/decorate/userInfo";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「用户信息」（type=userInfo）
*
* 渲染器本体就是会员页在用的同一个组件（pages/member/components/decorate/userInfo，
* 自旧 pages/member/components/decorate 移植）；本文件只是把它接入与首页/自定义页
* 相同的 MODULE_REGISTRY + DecorateModules 分发口径（适配器为我方新增）：
* 通用分发器传 { module}，会员组件收 { data}，在此转接。
*
* 此前未注册时，编辑器真实渲染（decoratePreview 走通用分发器）遇到会员模块
* 一律跳过，个人中心页（10012）中间画布整页空白（2026-11-19 用户报障）。
*/
export default function UserInfoModule({ module}: DecorateModuleProps) {
return <UserInfo data={module?? {}} />;
}
