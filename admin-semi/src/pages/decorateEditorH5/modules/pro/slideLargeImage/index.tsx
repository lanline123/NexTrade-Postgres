/**
 * 大图滑动 模块（契约自 view/admin editor/modules/mobilePro/slideLargeImage/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const picList = [
    {
        picId: 1424,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731388896OTbuADBoe1LEFefwJo.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731388896OTbuADBoe1LEFefwJo.png",
        picName: "FtohhFYy2fnPEqrcsqpSq5H53DzH"
    },
    {
        picId: 1425,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17313888965IIgWQGqsTYhoe7H7T.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17313888965IIgWQGqsTYhoe7H7T.png",
        picName: "FlVOfGJJyjhQmsiDGJViz_S2clyl"
    },
    {
        picId: 1424,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731388896OTbuADBoe1LEFefwJo.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731388896OTbuADBoe1LEFefwJo.png",
        picName: "FtohhFYy2fnPEqrcsqpSq5H53DzH"
    },
    {
        picId: 1425,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17313888965IIgWQGqsTYhoe7H7T.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17313888965IIgWQGqsTYhoe7H7T.png",
        picName: "FlVOfGJJyjhQmsiDGJViz_S2clyl"
    }
];

/** 与旧 mobilePro/slideLargeImage/index.ts slideLargeImageDefaults 逐字段一致（契约） */
export const slideLargeImageDefaults = {
    picList,
    rowNum: 1.5,
    slidingEffect: "auto",
    slidingSpeed: 25,
    picMarginTop: 0,
    picMargin: 10,
    picRadius: 6,
    picFillType: "cover",
    picTitleDesc: {
        show: 1,
        titleColor: "#ffffff",
        titleFontSize: 14,
        titleFontWeight: 400,
        descColor: "#ffffff",
        descFontSize: 12,
        descFontWeight: 0,
        fontPosition: "",
        textAlignment: "right",
        marginTop: 10,
        marginBottom: 10,
        marginLeft: 0,
        backgroundColor: ""
    },
    moduleTitle: {
        titleText: "",
        subTitleText: "",
        titlePosition: "top",
        textAlignment: "right",
        titleMarginTop: 0,
        titleMarginBottom: 0,
        titleMarginLeft: 0,
        titleColor: "#ffffff",
        titleFontSize: 16,
        titleFontWeight: 700,
        subFontTitleColor: "#ffffff",
        subFontTitleSize: 12,
        subFontTitleWeight: 700
    },
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "slideLargeImage",
    label: "大图滑动",
    icon: '<i class="h5-mod-ico">🌌</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 14,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(slideLargeImageDefaults)
});
