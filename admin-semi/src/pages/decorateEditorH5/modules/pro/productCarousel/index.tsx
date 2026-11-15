/**
 * 商品轮播 模块（契约自 view/admin editor/modules/mobilePro/productCarousel/index.ts 逐字段移植）
 *
 * defaults 直接采用共享 defaultProductCarousel（含演示项 defaultProductCarouselDemoItems，
 * 与原版 src/modules/index 单一来源一致）；原版 enabled: isPro 省略（我方即 pro 版）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultProductCarousel } from "../../../contract/sharedDefaults";

/** 与旧 mobilePro/productCarousel/index.ts productCarouselDefaults 逐字段一致（契约） */
export const productCarouselDefaults = defaultProductCarousel;

registerModule({
    type: "productCarousel",
    label: "商品轮播",
    icon: '<i class="h5-mod-ico">🎠</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 29,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productCarouselDefaults)
});
