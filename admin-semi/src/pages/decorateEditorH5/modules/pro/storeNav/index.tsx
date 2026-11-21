/**
 * 门店导航（mobilePro 桶）— 注册信息与 defaults 对齐原版 editor/modules/mobilePro/storeNav/index.ts
 * 原版 `enabled: () => isPro() && isStore()`：isPro 省略（我方即 pro 版）；isStore 为原版
 * VITE_IS_STORE 版本标志（contract/version 已按原版 env 口径实现，全量版 .env 置 1 恒真）。
 * 原版注册无 shopOnly——2026-11-21 随点单卡片复核一并删除我方误加的 shopOnly 并订正
 * isStore 误译（曾致平台管理员模块库里无此模块），报告 ~/workspace/storeordercard-audit/REPORT.md。
 * storeStyle/storeSource 的原版枚举 StoreNavStyle{One:1,Two:2} / StoreNavSource{Auto:1,Assign:2}
 * 以数值字面量落 defaults（types.ts 口径）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultSubTitle, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";
import { isStore } from "../../../contract/version";


export const storeNavDefaults = {
    subTitle: cloneDeep(defaultSubTitle),
    storeTitleStyle: {
        showTitle: 1,
        titleColor: "#1a1a1a",
        titleFontSize: 14,
        titleFontBold: 700
    },
    titlePaddingY: 0,
    titlePaddingX: 0,
    storeStyle: 1,
    storeSource: 1,
    storeNum: 10,
    storeList: [] as Array<Record<string, any>>,
    showRating: 1,
    showFollow: 1,
    showMinOrder: 1,
    showDeliveryFee: 1,
    showDistance: 1,
    storeGap: 15,
    storeBackgroundColor: "#ffffff",
    storePaddingY: 0,
    storePaddingX: 0,
    storeRadius: 0,
    productGap: 8,
    showProductName: 1,
    productNameRow: 1,
    showProductPrice: 1,
    moduleStyle: cloneDeep(defaultModuleStyle),
    contentStyle: { ...cloneDeep(defaultContentStyle), backgroundColor: "#ffffff", paddingTopBottom: 10 }
};

registerModule({
    type: "storeNav",
    label: "门店导航",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 23,
    enabled: () => isStore(),
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(storeNavDefaults)
});
