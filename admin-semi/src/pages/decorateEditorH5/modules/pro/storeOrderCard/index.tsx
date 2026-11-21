/**
 * 点单卡片（mobilePro 桶）— 注册信息与 defaults 对齐原版 editor/modules/mobilePro/storeOrderCard/index.ts
 * 原版 `enabled: () => isPro() && isStore()`：isPro 省略（我方即 pro 版）；isStore 为原版
 * VITE_IS_STORE 版本标志（contract/version 已按原版 env 口径实现，全量版 .env 置 1 恒真）。
 * 原版注册无 shopOnly——2026-11-21 复核删除我方误加的 shopOnly 并订正 isStore 误译
 * （曾致平台管理员模块库里无此模块），报告 ~/workspace/storeordercard-audit/REPORT.md。
 * 店铺端可见性：原版店铺端不见高级组；用户 2026-11-21 裁决本模块与门店导航/
 * 首焦轮播A2「全部的都可见」，注册以 shopVisible 显式豁免（原版有·我方改口径），
 * 报告同目录 REPORT2.md。
 * frame 默认 = defaultFrame 叠加本模块覆盖（moduleFrameDefault：圆角 8、内边距 10/10/10、图片边距 0）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";
import { isStore } from "../../../contract/version";


export const moduleFrameDefault = {
    boxRadius: 8,
    innerPadding: 10,
    innerPaddingLeftRight: 10,
    innerPaddingTopBottom: 10,
    imagePaddingTop: 0,
    imagePaddingBottom: 0,
    imagePaddingLeftRight: 0
};

export const buildDefaultCard = () => ({
    picId: 0,
    picThumb: "",
    picUrl: "",
    picName: "",
    title: "",
    desc: "",
    link: {} as Record<string, any>
});

/** 各模板对应卡片张数（原版 LENGTH_MAP） */
export const LENGTH_MAP: Record<number, number> = {
    1: 1,
    2: 2,
    3: 3,
    4: 3,
    5: 4,
    6: 5
};

export const storeOrderCardDefaults = {
    picType: 2,
    cardStyle: 1,
    boxStyle: 1,
    backgroundColor: "",
    cardBackground: "#ffffff",
    cardGap: 8,
    frame: { ...cloneDeep(defaultFrame), ...moduleFrameDefault },
    cardList: [buildDefaultCard(), buildDefaultCard()]
};

registerModule({
    type: "storeOrderCard",
    label: "点单卡片",
    icon: '<i class="h5-mod-ico">🧾</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 30,
    enabled: () => isStore(),
    // 店铺端亦可见：用户 2026-11-21 裁决「全部的都可见」（原版有·我方改口径，
    // 原版店铺端不见高级组），经 shopVisible 显式豁免，见 storeordercard-audit/REPORT2.md。
    shopVisible: true,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(storeOrderCardDefaults)
});
