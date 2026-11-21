/**
 * 天天特价（mobilePro 桶）— 注册信息与 defaults 对齐原版 editor/modules/mobilePro/specialOffer/index.ts
 * defaults 在本文件内逐字段复刻原版 specialOfferDefaults：
 * - productList 为原版演示商品 4 件（字段名原版写作 productList，注释里的 pruductList 仅为局部变量名）；
 * - dailyTime/specifyTime 按原版 utils/time 的 formatTime/getDaysLaterEndTime 语义近似
 *   （今日 8:00/20:00 时间戳、3 天后 23:59:59），与 mixedProductDisplay 同口径；
 * - moduleColorStyle 预设色（ModuleTheme）在 Preview/Settings 内本地复刻。
 * 原版 `enabled: isPro` 省略（我方本就是 pro 版编辑器）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";


function todayAt(hour: number): number {
    const d = new Date();
    d.setHours(hour, 0, 0, 0);
    return d.getTime();
}
function daysLaterEndTime(days: number): number {
    const d = new Date();
    d.setDate(d.getDate() + days);
    d.setHours(23, 59, 59, 0);
    return d.getTime();
}

export const specialOfferDefaults = {
    productSource: "products",
    // 「全部商品」来源（productSource="all"，我方新增，论坛 B 类 237824）的取数数量；
    // 原版契约无此字段与来源，仅 all 来源消费
    productNum: 100,
    productGroups: [] as Array<Record<string, any>>,
    productList: [
        {
            productId: 0,
            productSn: "",
            productName: "零食",
            productDesc: "",
            productPrice: 32.0,
            marketPrice: 0,
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17316338156QyBKObq7Wuw7KCqKa.jpg"
        },
        {
            productId: 0,
            productSn: "",
            productName: "饼干",
            productDesc: "",
            productPrice: 32.0,
            marketPrice: 0,
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731633815i7FmGX6OEWOBsQ3DUh.jpg"
        },
        {
            productId: 0,
            productSn: "",
            productName: "茶叶",
            productDesc: "",
            productPrice: 32.0,
            marketPrice: 0,
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731633815zoFfIRIsE4yJ0a5LJn.jpg"
        },
        {
            productId: 0,
            productSn: "",
            productName: "干货",
            productDesc: "",
            productPrice: 32.0,
            marketPrice: 0,
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731633815UdOFkntVJP4GXu9KsW.jpg"
        }
    ],
    moduleColorStyle: "orange",
    prodcutTitles: [] as Array<Record<string, any>>,
    showProdcutTitles: 1,
    moduleTitle: {
        title: "天天特价",
        subTitle: "每日精选",
        countdownTitle: "距结束",
        countdownType: "dailyCycle",
        dailyTime: [todayAt(8), todayAt(20)],
        specifyTime: daysLaterEndTime(3),
        topRightType: "countdown",
        buttonText: "立即抢购",
        buttonLink: {}
    },
    picScale: "1:1",
    picFillType: "cover",
    productWidth: 25,
    productRadius: 6,
    showProductName: 1,
    productNameSize: 14,
    productNameWeight: 400,
    showProductPrice: 1,
    priceWeight: 400,
    showSellingPoints: 1,
    moduleColor: {
        titleColor: "#ffffff",
        subTitleColor: "#ffffff",
        countdownColor: "#ffffff",
        countdownTitleColor: "#72B76C",
        buttonTextColor: "#ffffff",
        buttonGradientColorA: "",
        buttonGradientColorB: "",
        productNameColor: "#555555",
        sellingPointsColor: "#FB6D08"
    },
    moduleStyle: cloneDeep(defaultModuleStyle),
    contentStyle: cloneDeep(defaultContentStyle)
};

registerModule({
    type: "specialOffer",
    label: "天天特价",
    icon: '<i class="h5-mod-ico">💰</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 10,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(specialOfferDefaults)
});
