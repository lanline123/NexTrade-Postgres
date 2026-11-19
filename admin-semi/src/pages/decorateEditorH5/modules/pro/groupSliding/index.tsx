/**
 * 分组滑动 模块（契约自 view/admin editor/modules/mobilePro/groupSliding/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultBuyButtonStyle } from "../../../contract/sharedDefaults";

const groupList = [
    {
        title: "日用清洁热卖榜〉",
        subTitle: "快来跟大家一起买",
        titleLink: {
            link: "",
            title: ""
        },
        backgroundColor: "#72b76c",
        label: `今天直降十元 
                加量不加价 
                抢超值2大桶
                加量不加价 
                抢超值2大桶`,
        productList: [
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319190474y0XWFxO8GJZF0dg64.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319190474uihMnJYOkEzvGBeWv.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919047ZXFKxeDYmDn9zNQSA3.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319190476keLc7Twr4BYAVlUBQ.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/173191904794zD2Sy4XugNwMUBOW.jpg"
            }
        ]
    },
    {
        title: "家居日用热卖榜〉",
        subTitle: "正能家居 生活好帮手",
        titleLink: {
            link: "",
            title: ""
        },
        backgroundColor: "#818dfc",
        label: `今天直降十元 
                加量不加价 
                抢超值2大桶
                加量不加价 
                抢超值2大桶`,
        productList: [
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17319190597yMADwwbOKqaA9pN5O.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919059AeiNkel9O1UtD2eZeD.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919059ts7YHQo7rbPam9y8ZQ.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919059VFsxMuKvPxKoFNfq5y.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919059GV4LN9Xq2j5ACzoJqx.jpg"
            }
        ]
    },
    {
        title: "家居日用热卖榜〉",
        subTitle: "正能家居 生活好帮手",
        titleLink: {
            link: "",
            title: ""
        },
        backgroundColor: "#ff6600",
        label: `今天直降十元 
                加量不加价 
                抢超值2大桶
                加量不加价 
                抢超值2大桶`,
        productList: [
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071axvkfUDkQNFbIzF1iY.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071lOa0ZBx2mYlvq1lgjr.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071xUnObq2LokKPFecwXq.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071rwvbdR1BOrBne9YHqd.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "品牌旗舰电商版商超版洗洁精",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919071r7emytS94COnCxRtcw.jpg"
            }
        ]
    }
];

/** 与旧 mobilePro/groupSliding/index.ts groupSlidingDefaults 逐字段一致（契约） */
export const groupSlidingDefaults = {
    groupList,
    groupWidth: 70,
    groupRadius: 8,
    productPicRatio: 36,
    productPicRadius: 6,
    productPicRightRadius: 1,
    showRanking: 1,
    showProductLabel: 1,
    showProductName: 1,
    productNameRow: 1,
    productNameWeight: 400,
    productNameSize: 14,
    showProductPrice: 1,
    productPrice_weight: 700,
    productPrice_size: 14,
    productPriceMarginTop: 10,
    crossedOutPrice: 1,
    buyBtnStyle: defaultBuyButtonStyle,
    moduleColor: {
        titleSize: 20,
        subTitleSize: 14,
        titleColor: "#ffffff",
        subTitleColor: "#ffffff",
        titlePic: {
            picId: 1489,
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919039RxeYzkD0P8ByOeKiNa.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731919039RxeYzkD0P8ByOeKiNa.png",
            picName: "FljdiuyonglOZKlYhkv9ZcrSw44_"
        },
        productBackgroundColor: "#ffffff",
        productNameColor: "#333333",
        labelBackgroundColor: "#FFF1F6",
        labelTextColor: "#FB264D",
        productPriceColor: "#72b76c",
        crossedOutPriceColor: "#999999"
    },
    moduleStyle: defaultModuleStyle
};

registerModule({
    type: "groupSliding",
    label: "分组滑动",
    icon: '<i class="h5-mod-ico">🗃️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 4,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(groupSlidingDefaults)
});
