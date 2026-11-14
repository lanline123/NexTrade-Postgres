/**
 * 商品A1 模块（mobilePro；默认值直接复用 contract/sharedDefaults 的 defaultProductA1）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultProductA1 } from "../../../contract/sharedDefaults";

export const productA1Defaults = defaultProductA1;

registerModule({
    type: "productA1",
    label: "商品(样式A1)",
    icon: '<i class="h5-mod-ico">🛒</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 60,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaultProductA1)
});
