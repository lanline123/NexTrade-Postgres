/**
 * 新人二重礼 模块（契约自 view/admin editor/modules/mobilePro/newcomerCombo/index.ts 逐字段移植）
 *
 * defaults 直接采用共享 defaultNewcomerCombo（与原版 src/modules/index 单一来源一致）；
 * 原版 enabled: isPro 省略（我方即 pro 版）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultNewcomerCombo } from "../../../contract/sharedDefaults";

/** 与旧 mobilePro/newcomerCombo/index.ts newcomerComboDefaults 逐字段一致（契约） */
export const newcomerComboDefaults = defaultNewcomerCombo;

registerModule({
    type: "newcomerCombo",
    label: "新人二重礼",
    icon: '<i class="h5-mod-ico">🎁</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 26,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(newcomerComboDefaults)
});
