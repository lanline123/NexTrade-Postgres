/**
 * 我的服务 模块（契约自 view/admin editor/modules/mobileMember/myService/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/** 与旧 mobileMember/myService/View.vue 的 defaultModule 逐字段一致（契约；Settings.vue 内另一套 moduleTitle 初值以 index.ts 为准） */
export const myServiceDefaults = {
    moduleTitle: "服务管理",
    layoutStyle: 1,
    iconRadius: 50,
    serviceList: [] as Record<string, any>[],
    frame: {
        ...cloneDeep(defaultFrame),
        backgroundColor: "#ffffff",
        boxRadius: 10,
        innerPadding: 15,
        boxPadding: 10,
        boxPaddingTop: 10,
        boxPaddingBottom: 0
    }
};

registerModule({
    type: "myService",
    label: "我的服务",
    icon: '<i class="h5-mod-ico">🧰</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 9,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(myServiceDefaults)
});
