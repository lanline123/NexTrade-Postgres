/**
 * 店铺提示 模块（mobilePro；契约字段对齐 uniapp-react customPage/modules/shopTips）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

export const shopTipsDefaults = {
    picList: [
        { picUrl: "", picThumb: "", title: "正品保障", picLink: { link: "", title: "" } },
        { picUrl: "", picThumb: "", title: "极速发货", picLink: { link: "", title: "" } },
        { picUrl: "", picThumb: "", title: "售后无忧", picLink: { link: "", title: "" } }
    ],
    rowNum: 3,
    itemGap: 8,
    itemBackgroundColor: "",
    titleColor: "#333333",
    showDivider: 0,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "shopTips",
    label: "店铺提示",
    icon: '<i class="h5-mod-ico">💬</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 64,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(shopTipsDefaults)
});
