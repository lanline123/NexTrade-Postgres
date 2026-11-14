/**
 * 限时特惠 模块（mobilePro；默认值直接复用 contract/sharedDefaults 的 defaultLimitedSpecial）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultLimitedSpecial } from "../../../contract/sharedDefaults";

export const limitedSpecialDefaults = defaultLimitedSpecial;

registerModule({
    type: "limitedSpecial",
    label: "限量特惠",
    icon: '<i class="h5-mod-ico">🔥</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 54,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaultLimitedSpecial)
});
