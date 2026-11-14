/**
 * 店铺信息 模块（默认值字段对齐 uniapp-react 渲染端 customPage/modules/shopInfo/index.tsx 读取的 module 字段）
 *
 * 渲染端读取：style（ad-store_style__N，共 4 种）、backgroundDefault（0=自定义图，
 * 1~10=OSS 内置背景图序号）、customPic{picUrl,picThumb}（backgroundDefault=0 时用）。
 * 店铺数据（shopLogo/shopTitle/productCount/newProductCount/collectShop）由店铺页
 * 传入的 shopInfo 提供，不存模块数据。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

/** 与渲染端 shopInfo/index.tsx 读取字段逐项对应 */
export const shopInfoDefaults = {
    style: 1,
    backgroundDefault: 1,
    customPic: {
        picUrl: "",
        picThumb: ""
    }
};

registerModule({
    type: "shopInfo",
    label: "店铺信息",
    icon: '<i class="h5-mod-ico">🏬</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 35,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(shopInfoDefaults)
});
