/**
 * 商品滑动（mobilePro 桶）— 注册信息对齐原版 editor/modules/mobilePro/productSlide/index.ts
 * defaults 直接采用共享常量 defaultProductSlide（原版 productSlideDefaults 即该常量，单一来源）。
 * 原版 `enabled: isPro` 省略（我方本就是 pro 版编辑器）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultProductSlide } from "../../../contract/sharedDefaults";


registerModule({
    type: "productSlide",
    label: "商品滑动",
    icon: '<i class="h5-mod-ico">➡️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 28,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaultProductSlide)
});
