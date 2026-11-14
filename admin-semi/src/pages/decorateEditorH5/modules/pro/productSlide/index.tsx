/**
 * 商品滑动 模块（mobilePro；默认值直接复用 contract/sharedDefaults 的 defaultProductSlide）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultProductSlide } from "../../../contract/sharedDefaults";

export const productSlideDefaults = defaultProductSlide;

registerModule({
    type: "productSlide",
    label: "商品滑动",
    icon: '<i class="h5-mod-ico">➡️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 63,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaultProductSlide)
});
