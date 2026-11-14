/**
 * 商品详情(样式A3) 模块
 * 详情页固定渲染、此模块为详情装修页(pageType=8)契约占位，字段待与详情页对接。
 * 基础字段沿用 modules/generic/simpleModules.tsx 现注册 defaults（productTitle/description/price/buttonText/pic），
 * 扩展 style 样式变体与 showName/showPrice/showBrief/showMarketPrice/showButton 显隐及配色字段。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const productDetailA3Defaults = {
    style: 3,
    productTitle: "商品标题",
    description: "商品介绍",
    price: 199,
    marketPrice: 299,
    buttonText: "立即购买",
    pic: { picUrl: "", picThumb: "" },
    showName: 1,
    showPrice: 1,
    showBrief: 1,
    showMarketPrice: 1,
    showButton: 1,
    nameColor: "",
    priceColor: "",
    buttonColor: "",
    backgroundColor: "",
    frame: defaultFrame
};

registerModule({
    type: "productDetailA3",
    label: "商品详情(样式A3)",
    icon: '<i class="h5-mod-ico">📄</i>',
    url: "mobileProductDetail",
    platform: "mobile",
    order: 97,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productDetailA3Defaults)
});
