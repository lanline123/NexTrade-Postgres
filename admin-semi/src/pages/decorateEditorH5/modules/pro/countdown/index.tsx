/**
 * 倒计时 模块（契约自 view/admin editor/modules/mobilePro/countdown/index.ts 逐字段移植）
 * 注：原版分组商品列表键名为 pruductList（原版拼写），此处保持原样；
 * dailyTime/specifyTime 原版为运行时动态值（formatTime(今日, 20点) / 1 天后结束时刻），此处以同口径函数生成。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const pruductList = [
    {
        productId: 0,
        productSn: "",
        productName: "麦片",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662JqyhZ6f1BCsWeUJlgj.jpg"
    },
    {
        productId: 0,
        productSn: "",
        productName: "巧克力棒",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662cOmZ28i8gKl6JiUBad.jpg"
    },
    {
        productId: 0,
        productSn: "",
        productName: "汤达人",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662AztoZ2OADHp1b8vUoQ.jpg"
    },
    {
        productId: 0,
        productSn: "",
        productName: "士力架",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662O7VZIzaFtItS4OvAOR.jpg"
    }
];
/** 原版 formatTime(new Date(), 20)：今日 20:00 的时间戳 */
function todayAt20(): number {
    const d = new Date();
    d.setHours(20, 0, 0, 0);
    return d.getTime();
}
/** 原版 getDaysLaterEndTime(1)：1 天后结束时刻的时间戳 */
function oneDayLaterEnd(): number {
    return Date.now() + 24 * 60 * 60 * 1000;
}

/** 与旧 mobilePro/countdown/index.ts countdownDefaults 逐字段一致（契约） */
export const countdownDefaults = {
    pruductList,
    moduleColorStyle: "orange",
    moduleTitle: {
        title: "限时抢购",
        subTitle: "超低价格，史无前例",
        moreText: "向左滑动查看更多",
        titleLink: { link: "", title: "" }
    },
    countdownData: {
        showCountdown: 1,
        countdownBackgroundPic: "",
        countdownType: "dailyCycle",
        dailyTime: todayAt20(),
        specifyTime: oneDayLaterEnd(),
        timeContent: ["天", "时", "分", "秒"],
        marginTop: 50,
        countdownPicBottom: 30
    },
    rowNum: 3,
    picScale: "1:1",
    picFillType: "cover",
    textAlignment: "center",
    productPadding: 5,
    productRadius: 5,
    picBottomRadius: 1,
    showProductName: 0,
    showProductPrice: 1,
    priceWeight: 700,
    crossedOutPrice: 1,
    moduleColor: {
        titleColor: "#ffffff",
        subTitleColor: "#ffffff",
        tipsColor: "#fafafa",
        productBackgroundColor: "#ffffff",
        productNameColor: "#333333",
        productPriceColor: "#72b76c",
        crossedOutPriceColor: "#999999",
        countdownColor: "#f7f7f7"
    },
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "countdown",
    label: "倒计时",
    icon: '<i class="h5-mod-ico">⏰</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 5,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(countdownDefaults)
});
