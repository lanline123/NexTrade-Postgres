/**
 * 新人专享二重礼 模块（mobilePro；默认值直接复用 contract/sharedDefaults 的 defaultNewcomerCombo）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultNewcomerCombo } from "../../../contract/sharedDefaults";

export const newcomerComboDefaults = defaultNewcomerCombo;

registerModule({
    type: "newcomerCombo",
    label: "新人专享",
    icon: '<i class="h5-mod-ico">🎁</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 57,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaultNewcomerCombo)
});
