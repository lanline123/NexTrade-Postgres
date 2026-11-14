/**
 * 固定功能区域 模块（契约依据：uniapp-react pages/member/components/decorate/fixedFunctionalArea/index.tsx）
 * 渲染端不读任何业务字段：内容固定为账期支付卡（TermPayCard，登录且有授信时显示），仅 frame 驱动外壳样式。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const fixedFunctionalAreaDefaults = {
    frame: { ...defaultFrame, backgroundColor: "transparent", boxRadius: 0, innerPadding: 0, boxPadding: 5, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "fixedFunctionalArea",
    label: "固定功能区域",
    icon: '<i class="h5-mod-ico">🧰</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 90,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(fixedFunctionalAreaDefaults)
});
