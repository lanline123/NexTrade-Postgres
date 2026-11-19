/**
 * 组合推荐 模块（契约自 view/admin editor/modules/mobilePro/comboSuggestions/index.ts 逐字段移植）
 * 注：原版分组商品列表键名为 pruductList（原版拼写），此处保持原样。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle } from "../../../contract/sharedDefaults";

const comboGroups = [
    {
        titleImage: {
            picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744JPyV2kPmhJjwi9b1OD.png",
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744JPyV2kPmhJjwi9b1OD.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            picId: 1429,
            picName: "FpMGh7lhbRC26udZs5OVGsD1MQkm"
        },
        groupTitle: "限时特惠",
        groupSubTitle: "心动福利",
        groupDesc: "限时！西冷牛排降20.1元",
        groupColor: "rgba(218, 79, 51, 1)",
        groupType: "image",
        picList: [
            {
                picId: 1428,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744Z36HLJIkKupT78pAPB.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
                picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744Z36HLJIkKupT78pAPB.jpg",
                picName: "FmBvbpIyBQcPAjaR1tpYu4NSfM2L",
                picTitle: "禽蛋抢先购"
            },
            {
                picId: 1427,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744G6SecTiZ2YnnqTOrzO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
                picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744G6SecTiZ2YnnqTOrzO.jpg",
                picName: "Fk9ihEJjXDwwG174KZcjuaQeyGus",
                picTitle: "清洁好帮手"
            },
            {
                picId: 1426,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744OYBNIbPBqdFUwc8YMO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
                picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485744OYBNIbPBqdFUwc8YMO.jpg",
                picName: "FlX7c5fV9sbgl9QZ0i0z1K1DK6DB",
                picTitle: "精选好食材"
            }
        ],
        pruductList: [],
        moreLink: {
            link: {
                link: "",
                title: ""
            },
            linkTitle: ""
        }
    },
    {
        titleImage: {
            picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17314857564qYuFM87aXbimtojrK.png",
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17314857564qYuFM87aXbimtojrK.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            picId: 1434,
            picName: "FqaBzM4nqSLzx4ydDgi3SfbS5R3g"
        },
        groupTitle: "生鲜惠选",
        groupSubTitle: "榜单推荐",
        groupDesc: "猫山王榴莲冰皮月饼香甜爆浆",
        groupColor: "rgba(177, 123, 57, 1)",
        groupType: "product",
        picList: [],
        pruductList: [
            {
                productId: 0,
                productSn: "",
                productName: "刺身拼盘",
                productDesc: "",
                productPrice: 39.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485756wtyOOnqoeEn6W2XfNP.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "冰鲜剥皮鱼",
                productDesc: "",
                productPrice: 39.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485756sGPXRzI7o84S5HAPo5.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "北极甜虾",
                productDesc: "",
                productPrice: 39.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485756JsgKabEE4BO09iRqnc.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "秋刀鱼",
                productDesc: "",
                productPrice: 39.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485756ivBG9CydBhi21do1xk.jpg"
            }
        ],
        moreLink: {
            link: {
                link: "",
                title: ""
            },
            linkTitle: ""
        }
    },
    {
        titleImage: {
            picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485768izrm5sZYVmQOne5yYf.png",
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485768izrm5sZYVmQOne5yYf.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            picId: 1439,
            picName: "FrEc7JTpoNjpEJR8HlE4Klv6trKM"
        },
        groupTitle: "全球好物",
        groupSubTitle: "大牌严选",
        groupDesc: "七夕礼~欧舒丹明细沐浴礼盒",
        groupColor: "rgba(36, 78, 179, 1)",
        groupType: "product",
        picList: [],
        pruductList: [
            {
                productId: 0,
                productSn: "",
                productName: "牙膏",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485768ECo8ixzZvZVOLDsJf3.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "卫生巾",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485768PFd7qRdzskII6k0ZTU.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "卫生巾",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485768yn1Oq5lqvNyx7N8gbE.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "沐浴露",
                productDesc: "",
                productPrice: 80.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485768bH293zqhEdmoNbO7VQ.jpg"
            }
        ],
        moreLink: {
            link: {
                link: "",
                title: ""
            },
            linkTitle: ""
        }
    },
    {
        titleImage: {
            picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485780Z15LILaEOLfudumi3H.png",
            picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485780Z15LILaEOLfudumi3H.png?x-oss-process=image/resize,m_pad,h_200,h_200",
            picId: 1441,
            picName: "FlF_0eUIigJ19i2n6VOTIhZuMn0G"
        },
        groupTitle: "逛出新奇",
        groupSubTitle: "上新日历",
        groupDesc: "颗颗饱满！甜！巨峰葡萄~",
        groupColor: "rgba(99, 49, 119, 1)",
        groupType: "product",
        picList: [],
        pruductList: [
            {
                productId: 0,
                productSn: "",
                productName: "摆件",
                productDesc: "",
                productPrice: 55.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485780O9GnV2rCLcOodydJAb.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "桌面垃圾桶",
                productDesc: "",
                productPrice: 25.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17314857806ABCknloual53A5XH0.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "垃圾袋",
                productDesc: "",
                productPrice: 15.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485780y9OjupFgDDLmI31GhJ.jpg"
            },
            {
                productId: 0,
                productSn: "",
                productName: "脸盆",
                productDesc: "",
                productPrice: 25.0,
                marketPrice: 0,
                picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731485780Okip8X3FFefKxhz8OG.jpg"
            }
        ],
        moreLink: {
            link: {
                link: "",
                title: ""
            },
            linkTitle: ""
        }
    }
];

/** 与旧 mobilePro/comboSuggestions/index.ts comboSuggestionsDefaults 逐字段一致（契约） */
export const comboSuggestionsDefaults = {
    comboGroups,
    regularPriceColor: "#e84a40",
    regularPriceBgColor: "",
    regularPriceRadius: 24,
    regularPriceSize: 14,
    saleTagBackgroundColorA: "#f8d7a0",
    saleTagBackgroundColorB: "#ffffff",
    saleTagFontColor: "#83441c",
    salePriceBackgroundColor: "#e73a68",
    salePriceFontColor: "#ffffff",
    salePriceDecimal: 0,
    imageRadius: 6,
    soldOutIcon: 1,
    backgroundGradientType: "upDown",
    gradientColorA: "#ffeadf",
    gradientColorB: "#ffffff",
    groupTitleColor: "#000000",
    bannerTittleColor: "#ffffff",
    bannerDotColor: "#ffffff",
    groupRadius: 10,
    groupShadow: 0,
    moduleStyle: defaultModuleStyle,
    carouselDirection: "horizontal",
    autoPlay: 1,
    interval: 5
};

registerModule({
    type: "comboSuggestions",
    label: "组合推荐",
    icon: '<i class="h5-mod-ico">💡</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 13,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(comboSuggestionsDefaults)
});
