/**
 * 商品A1 模块（契约自 view/admin editor/modules/mobilePro/productA1/index.ts 逐字段移植）
 *
 * defaults 直接采用共享 defaultProductA1（与原版 src/modules/index 单一来源一致）；
 * 原版 enabled: isPro 省略（我方即 pro 版）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultProductA1 } from "../../../contract/sharedDefaults";

/** 与旧 mobilePro/productA1/index.ts productA1Defaults 逐字段一致（契约） */
export const productA1Defaults = defaultProductA1;

registerModule({
    type: "productA1",
    label: "商品A1",
    icon: '<i class="h5-mod-ico">🛒</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 8,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productA1Defaults)
});
