/**
 * 固定功能区 模块（契约自 view/admin editor/modules/mobileMember/fixedFunctionalArea/index.ts 逐字段移植）
 *
 * 系统注入模块：原版注册 visible: false（不在添加面板出现，由会员页 normalize 注入，fixed 固定模块）；
 * defaults 仅 frame（index.ts 口径；原版 Settings.vue 内另一套 frame 初值与 index.ts 不一致，以 index.ts 为准——审计已注）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobileMember/fixedFunctionalArea/index.ts fixedFunctionalAreaDefaults 逐字段一致（契约） */
export const fixedFunctionalAreaDefaults = {
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "",
        boxRadius: 0,
        innerPadding: 0,
        boxPadding: 5,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "fixedFunctionalArea",
    label: "固定功能区",
    icon: '<i class="h5-mod-ico">📌</i>',
    url: "mobileMember",
    platform: "mobile",
    visible: false,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(fixedFunctionalAreaDefaults)
});
