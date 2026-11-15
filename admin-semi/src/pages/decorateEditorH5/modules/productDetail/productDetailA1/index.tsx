/** 商品详情 A1（标准风）（移植自 ref-editor mobileProductDetail/productDetailA1/index.ts） */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";
import { createProductDetailDefaults } from "../shared/defaults";

const defaults = createProductDetailDefaults("A1");

registerModule({
    type: "productDetailA1",
    label: "商品详情A1",
    icon: '<i class="h5-mod-ico">商</i>',
    url: "mobileProductDetail",
    platform: "mobile",
    order: 1,
    exclusive: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(defaults)
});
