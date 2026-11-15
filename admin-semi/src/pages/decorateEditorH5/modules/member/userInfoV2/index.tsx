/**
 * 用户信息2 模块（契约自 view/admin editor/modules/mobileMember/userInfoV2/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobileMember/userInfoV2/View.vue 的 defaultModule 逐字段一致（契约） */
export const userInfoV2Defaults = {
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        boxRadius: 0,
        innerPadding: 16,
        boxPadding: 0,
        boxPaddingTop: 0,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "userInfoV2",
    label: "用户信息2",
    icon: '<i class="h5-mod-ico">👤</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 2,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(userInfoV2Defaults)
});
