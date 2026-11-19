/**
 * 商品提示 模块（契约自 view/admin editor/modules/mobilePro/shopTips/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

// 演示提示项是 defaultModule 字面量的一部分（旧 View.vue 的 defaultPicList 常量，契约冻结；第三项 picUrl 尾部多一个 g 为原版原样）
const defaultPicList = [
    {
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779263739npDO706Su0cRzKK3ar.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779263739npDO706Su0cRzKK3ar.png",
        picTitle: "最快30分钟送达",
        picLink: {}
    },
    {
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779263739N0N6CeZ40H7KJY9vRb.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779263739N0N6CeZ40H7KJY9vRb.png",
        picTitle: "满69元包邮",
        picLink: {}
    },
    {
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779263739qL8q3AHR1CcdoNv4sD.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/official/img/pc/202605/1779263739qL8q3AHR1CcdoNv4sD.pngg",
        picTitle: "品质保障",
        picLink: {}
    }
];

/** 与旧 mobilePro/shopTips/index.ts shopTipsDefaults 逐字段一致（契约） */
export const shopTipsDefaults = {
    picList: defaultPicList,
    rowNum: 3,
    itemGap: 0,
    showDivider: 0,
    titleColor: "#00C455",
    itemBackgroundColor: "",
    moduleStyle: { ...cloneDeep(defaultModuleStyle), boxPaddingTop: 5, boxPaddingBottom: 5, boxPadding: 10 },
    contentStyle: defaultContentStyle
};

registerModule({
    type: "shopTips",
    label: "商品提示",
    icon: '<i class="h5-mod-ico">💬</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 24,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(shopTipsDefaults)
});
