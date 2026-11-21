/**
 * 商品分组 模块（契约自 view/admin editor/modules/mobilePro/productGrouping/index.ts 逐字段移植）
 *
 * 演示分组数据为原版 index.ts 字面量的一部分（契约冻结），此处内联（4 组同构，
 * 以工厂函数展开，字段/数值与原版逐一一致）。原版 enabled: isPro 省略（我方即 pro 版）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultBuyButtonStyle, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const demoItem = (productPrice: number, marketPrice: number, picThumb: string) => ({
    productId: 0,
    productSn: "",
    productName: "商品的大标题雨价格在后台他会自动调用读取",
    productDesc: "",
    productPrice,
    marketPrice,
    picThumb
});

const demoGroupList = (): Array<Record<string, any>> => [
    demoItem(88.8, 98.8, "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732004872pw39D1oGqdAaZMpvyo.jpg"),
    demoItem(56.8, 68.88, "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732004872f6EcZnbAqdW5kIGzxL.jpg"),
    demoItem(48.8, 69.9, "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732004872o3s8aqPGdICr1DOCdB.jpg"),
    demoItem(48.8, 69.9, "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732004872Mry8L9BWOCiawPhp72.jpg"),
    demoItem(32.0, 39.0, "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732004872yPYr4zSj7AXrfw9oON.jpg"),
    demoItem(32.0, 39.0, "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732004872NlX49KeGeYSN4Tw0wa.jpg")
];

const demoGroup = (title: string, subTitle: string) => ({
    title,
    subTitle,
    titleLink: {},
    bannerPic: {},
    bannerLink: {},
    productList: demoGroupList()
});

/** 与旧 mobilePro/productGrouping/index.ts productGroupingDefaults 逐字段一致（契约） */
export const productGroupingDefaults = {
    productSource: "products",
    // 「全部商品」来源（productSource="all"，我方新增，论坛 B 类 237824）的取数数量；
    // 原版契约无此字段与来源，仅 all 来源消费，其余来源忽略
    productNum: 100,
    productGroups: [demoGroup("新品", "每月上新"), demoGroup("热卖", "特别精选"), demoGroup("折扣", "便宜好货"), demoGroup("预售", "提前锁定")],
    groupList: [],

    productCardStyle: 1,
    listStyle: 2,
    picScale: "1:1",
    picFillType: "cover",

    cardMargin: 6,
    cardPadding: 0,
    cardRadius: 5,
    picBottomRadius: 1,

    showProductName: 1,
    productNameRow: 1,
    productNameWeight: 400,
    productNameSize: 14,

    showProductPrice: 1,
    productPricePic: "",
    productPriceMarginTop: 10,
    productPriceWeight: 400,
    productPrice_size: 14,

    crossedOutPrice: 1,

    buyBtnStyle: cloneDeep(defaultBuyButtonStyle),

    bottomMoreBtn: "text",
    bottomMoreBtnText: "查看更多",
    bottomMoreBtnImage: "",

    productColor: {
        infoTopPadding: 5,
        infoBottomPadding: 5,
        infoLeftRightPadding: 3,
        backgroundColor: "#ffffff",
        titleColor: "#555555",
        priceColor: "#72B76C",
        crossedOutPriceColor: "#999999",
        moreBtnBgColor: "#ffffff",
        moreBtnBorderColor: "#eeeeee",
        moreBtnTextColor: "#333333"
    },
    groupTitle: {
        marginLeftRight: 0,
        showTitle: 1,
        bigTitleFontSize: 16,
        titleColor: "#444444",
        activeTitleColor: "#72B76C",
        showSubTitle: 1,
        titleFontSize: 14,
        subTitleColor: "#999999",
        activeSubTitleColor: "#ffffff",
        subTitleGradientType: "diagonal",
        gradientColorA: "#72b76c",
        gradientColorB: "#72b76c",
        subTitleRadius: 15,
        splitLineColor: "#DADADA"
    },
    moduleStyle: cloneDeep(defaultModuleStyle),
    contentStyle: cloneDeep(defaultContentStyle)
};

registerModule({
    type: "productGrouping",
    label: "商品分组",
    icon: '<i class="h5-mod-ico">🗃️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 3,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(productGroupingDefaults)
});
