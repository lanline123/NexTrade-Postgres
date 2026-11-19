import MemberAdImage from "@/pages/member/components/decorate/adImage";
import type { DecorateModuleProps} from "@/pages/customPage/modules/registry";

/**
* 会员中心「广告图」（type=adImage）
* 渲染器本体 = 会员页同一组件（pages/member/components/decorate/adImage）；
* 本文件为接入通用 MODULE_REGISTRY 分发的适配器（我方新增），口径见 userInfo/index.tsx。
* 注意与首页的 imageAd（type=imageAd）是两个不同模块、数据契约不同，不可混用。
*/
export default function AdImageModule({ module}: DecorateModuleProps) {
return <MemberAdImage data={module?? {}} />;
}
