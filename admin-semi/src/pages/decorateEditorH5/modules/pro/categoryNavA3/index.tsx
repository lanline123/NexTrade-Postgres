/**
 * 类目导航A3 模块（契约自 view/admin editor/modules/mobilePro/categoryNavA3/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultSubTitle, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const categoryList = [
    {
        picId: 1519,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732608267mJFZA5uFuT5974JXl8.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732608267mJFZA5uFuT5974JXl8.png",
        picName: "FgisG78Lwl4UG3y5fBiEUnLrxIV",
        picTitle: "诚信经营",
        picDesc: "诚信经营"
    },
    {
        picId: 1522,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732608267sJb7DHOuVX8896buhT.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732608267sJb7DHOuVX8896buhT.png",
        picName: "FpnzuzjBafleMMwgzc5MvNITxVHj",
        picTitle: "品质保障",
        picDesc: "诚信经营"
    },
    {
        picId: 1521,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732608267yXWHECJxn75eqyk60Q.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1732608267yXWHECJxn75eqyk60Q.png",
        picName: "Fh56a7Prcs9wghXkxYtAHGlF3_uT1",
        picTitle: "贴心服务",
        picDesc: "诚信经营"
    },
    {
        picId: 1520,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17326082679hhyPXOj9i02unfUgw.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17326082679hhyPXOj9i02unfUgw.png",
        picName: "Fn5u9A8vmu2DAG404sp8z_016zdK",
        picTitle: "购物无忧",
        picDesc: "诚信经营"
    }
];
const logo = {
    picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202409/1727056331nhxqtK8Mm5u8MXokoI.png",
    picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202409/1727056331nhxqtK8Mm5u8MXokoI.png?x-oss-process=image/resize,m_pad,h_200,h_200",
    picId: 1221,
    picName: "logoWhite"
};

/** 与旧 mobilePro/categoryNavA3/index.ts categoryNavA3Defaults 逐字段一致（契约） */
export const categoryNavA3Defaults = {
    logoPic: logo,
    categoryList,
    listStyleType: "list",
    rowNum: 4,
    imageBackgroundColor: "",
    picFillType: "cover",
    picPercent: 90,
    radius: 200,
    categoryPadding: 10,
    subTitle: defaultSubTitle,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryNavA3",
    label: "类目导航A3",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 21,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryNavA3Defaults)
});
