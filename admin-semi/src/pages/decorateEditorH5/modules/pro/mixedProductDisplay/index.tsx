/**
 * 商品混排 模块（契约自 view/admin editor/modules/mobilePro/mixedProductDisplay/index.ts 逐字段移植）
 *
 * 演示分组/演示图为原版 index.ts 字面量的一部分（契约冻结），此处内联。
 * dailyTime/specifyTime 原版由 utils/time 的 formatTime/getDaysLaterEndTime 生成：
 * formatTime(date, 8|20) = 当天 8:00/20:00 时间戳；getDaysLaterEndTime(3) = 3 天后时间戳，
 * 本实现以等价本地计算承接（口径见 h5-step8-proB.md）。
 * 原版 enabled: isPro 省略（我方即 pro 版）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const todayAt = (hour: number): number => {
    const d = new Date();
    d.setHours(hour, 0, 0, 0);
    return d.getTime();
};
const daysLaterEndTime = (days: number): number => {
    const d = new Date();
    d.setDate(d.getDate() + days);
    d.setHours(23, 59, 59, 999);
    return d.getTime();
};

const demoPic = {
    picId: 1454,
    picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293Kfvp2BXElAzXCOY7pq.png?x-oss-process=image/resize,m_pad,h_200,h_200",
    picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293Kfvp2BXElAzXCOY7pq.png",
    picName: "Fi_ZwwrzwDQddtmqTBCAyht4dPI"
};

const demoProduct = (productName: string, picThumb: string) => ({
    productId: 0,
    productSn: "",
    productName,
    productDesc: "",
    productPrice: 39.0,
    marketPrice: 1999.0,
    picThumb
});

const demoGroups = [
    {
        groupTitle: "限时福利",
        groupSubTitle: "直触底价 先到先得",
        groupType: "product",
        pruductList: [
            demoProduct("德宝纸巾", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293KF9TjvpDKHOATMR3H7.jpg"),
            demoProduct("薯片", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650294a4EsPq1zkYme6p5OhC.jpg"),
            demoProduct("饼干", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650294hYUnaJSx7UhsjhhGEi.jpg"),
            demoProduct("奶粉", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293zLV7bKUMAFWH6hrM7m.jpg")
        ],
        moreLink: { link: { link: "", title: "" }, linkTitle: "" }
    },
    {
        groupTitle: "发现好货",
        groupSubTitle: "精选推荐 多买多省",
        groupType: "product",
        pruductList: [
            demoProduct("高露洁牙膏", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17316502936Ow5WaZszFnuYuGzB8.jpg"),
            demoProduct("黑人牙膏", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293Tx6p8XKkDJPhDOOAT3.jpg"),
            demoProduct("高露洁牙膏", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293yDagFw4VE9FG1oJSnW.jpg"),
            demoProduct("黑人牙膏", "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650294KeYpvFBkgkloWdbqX7.jpg")
        ],
        moreLink: { link: { link: "", title: "" }, linkTitle: "" }
    }
];

/** 与旧 mobilePro/mixedProductDisplay/index.ts mixedProductDisplayDefaults 逐字段一致（契约） */
export const mixedProductDisplayDefaults = {
    productGroups: demoGroups,
    rightPic: demoPic,
    showCountdown: 1,
    countdownType: "dailyCycle",
    dailyTime: [todayAt(8), todayAt(20)],
    specifyTime: daysLaterEndTime(3),
    timeContent: ["时", "分", "秒"],

    picScale: "1:1",
    picFillType: "cover",
    productRadius: 6,
    morePosition: "top",

    showProductPrice: 1,
    priceWeight: 400,
    crossedOutPrice: 1,
    productSales: 0,

    moduleColor: {
        titleColor: "#171B1E",
        sellingPointColor: "#999999",
        countdownBackgroundColor: "#72B76C",
        countdownColor: "#ffffff",
        priceBackgroundColor: "#72B76C",
        priceColor: "#ffffff",
        crossedOutPriceColor: "#999999",
        lineColor: "#eeeeee"
    },

    moduleStyle: cloneDeep(defaultModuleStyle),
    contentStyle: cloneDeep(defaultContentStyle)
};

registerModule({
    type: "mixedProductDisplay",
    label: "商品混排",
    icon: '<i class="h5-mod-ico">🧩</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 9,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(mixedProductDisplayDefaults)
});
