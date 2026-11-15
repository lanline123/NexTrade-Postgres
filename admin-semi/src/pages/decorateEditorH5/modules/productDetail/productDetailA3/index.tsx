/** 商品详情 A3（跨境风）（移植自 ref-editor mobileProductDetail/productDetailA3/index.ts） */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";
import { productDetailA3Defaults } from "../shared/defaults";

registerModule({
    type: "productDetailA3",
    label: "商品详情A3",
    icon: '<i class="h5-mod-ico">商</i>',
    url: "mobileProductDetail",
    platform: "mobile",
    order: 3,
    exclusive: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productDetailA3Defaults)
});
