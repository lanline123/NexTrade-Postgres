/**
 * 限时特惠 模块（契约自 view/admin editor/modules/mobilePro/limitedSpecial/index.ts 逐字段移植）
 *
 * defaults 直接采用共享 defaultLimitedSpecial（与原版 src/modules/index 的
 * defaultLimitedSpecial 单一来源一致）；原版 enabled: isPro 省略（我方即 pro 版）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultLimitedSpecial } from "../../../contract/sharedDefaults";

/** 与旧 mobilePro/limitedSpecial/index.ts limitedSpecialDefaults 逐字段一致（契约） */
export const limitedSpecialDefaults = defaultLimitedSpecial;

registerModule({
    type: "limitedSpecial",
    label: "限时特惠",
    icon: '<i class="h5-mod-ico">🔥</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 27,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(limitedSpecialDefaults)
});
