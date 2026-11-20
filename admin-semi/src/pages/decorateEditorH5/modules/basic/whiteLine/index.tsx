/**
 * 分割线 模块（契约自 view/admin editor/modules/mobile/whiteLine/index.ts 逐字段移植；
 * 唯 lineType 默认值经用户指示改口径，见 whiteLineDefaults 注释）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/**
 * 与旧 mobile/whiteLine/View.vue 的 defaultModule 逐字段一致（契约），一处例外：
 * lineType 原版默认 1（空白），2026-11-21 用户指示改为默认 2（分割线）——
 * 模块名即「分割线」，插入后应直接是线；且与 PC 编辑器 pc-divider 的默认
 * （lineType: 2）对齐。此默认值同时是新插入模块的初始数据与编辑器
 * 预览/设置面对存量缺字段数据的合并兜底来源（单一来源，勿在他处另写）。
 */
export const whiteLineDefaults = {
    lineType: 2,
    lineStyle: 1,
    frame: defaultFrame
};

registerModule({
    type: "whiteLine",
    label: "分割线",
    icon: '<i class="h5-mod-ico">➖</i>',
    url: "mobile",
    platform: "mobile",
    order: 7,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(whiteLineDefaults)
});
