/**
 * 商品搜索 模块（契约自 view/admin editor/modules/mobile/searchBar/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame, defaultIcoPic } from "../../../contract/sharedDefaults";
import { isShopEnd } from "../../../contract/version";

/** 与旧 mobile/searchBar/index.ts searchBarDefaults 逐字段一致（契约） */
export const searchBarDefaults = {
    text: "搜索商品",
    icoPic: defaultIcoPic,
    frame: defaultFrame
};

registerModule({
    type: "searchBar",
    label: "商品搜索",
    icon: '<i class="h5-mod-ico">🔍</i>',
    url: "mobile",
    platform: "mobile",
    order: 14,
    // 店铺端（shopId 非空且非 "0"）不显示，与原版 enabled: () => !isShopEnd() 一致
    enabled: () => !isShopEnd(),
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(searchBarDefaults)
});
