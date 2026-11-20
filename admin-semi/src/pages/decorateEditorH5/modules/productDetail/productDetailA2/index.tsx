/** 商品详情 A2（门店风）（移植自 ref-editor mobileProductDetail/productDetailA2/index.ts） */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";
import { productDetailA2Defaults } from "../shared/defaults";

registerModule({
    type: "productDetailA2",
    label: "商品详情A2",
    desc: "配置门店风商品详情页的图片、标题、价格与购买栏样式",
    icon: '<i class="h5-mod-ico">商</i>',
    url: "mobileProductDetail",
    platform: "mobile",
    order: 2,
    exclusive: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productDetailA2Defaults)
});
