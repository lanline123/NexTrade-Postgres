/**
 * 商品轮播 模块（mobilePro；默认值直接复用 contract/sharedDefaults 的 defaultProductCarousel）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultProductCarousel } from "../../../contract/sharedDefaults";

export const productCarouselDefaults = defaultProductCarousel;

registerModule({
    type: "productCarousel",
    label: "商品轮播",
    icon: '<i class="h5-mod-ico">🎠</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 61,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaultProductCarousel)
});
