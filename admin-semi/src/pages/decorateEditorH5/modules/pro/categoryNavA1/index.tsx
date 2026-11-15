/**
 * 类目导航A1 模块（契约自 view/admin editor/modules/mobilePro/categoryNavA1/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultAnimation, defaultSubTitle, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const categoryList = [
    {
        picId: 1519,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1732608267mJFZA5uFuT5974JXl8.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1732608267mJFZA5uFuT5974JXl8.png",
        picName: "FgisG78Lwl4UG3y5fBiEUnLrxIV",
        picTitle: "诚信经营"
    },
    {
        picId: 1522,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1732608267sJb7DHOuVX8896buhT.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1732608267sJb7DHOuVX8896buhT.png",
        picName: "FpnzuzjBafleMMwgzc5MvNITxVHj",
        picTitle: "品质保障"
    },
    {
        picId: 1521,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1732608267yXWHECJxn75eqyk60Q.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1732608267yXWHECJxn75eqyk60Q.png",
        picName: "Fh56a7Prcs9wghXkxYtAHGlF3_uT1",
        picTitle: "贴心服务"
    },
    {
        picId: 1520,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/17326082679hhyPXOj9i02unfUgw.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/17326082679hhyPXOj9i02unfUgw.png",
        picName: "Fn5u9A8vmu2DAG404sp8z_016zdK",
        picTitle: "购物无忧"
    }
];

/** 与旧 mobilePro/categoryNavA1/index.ts categoryNavA1Defaults 逐字段一致（契约） */
export const categoryNavA1Defaults = {
    categoryList,
    listStyleType: "list",
    rowNum: 4,
    animation: defaultAnimation,
    imageBackgroundColor: "",
    picFillType: "cover",
    picPercent: 90,
    radius: 200,
    categoryPadding: 10,
    subTitle: defaultSubTitle,
    showSplitLine: 1,
    radiusStyle: 1,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryNavA1",
    label: "类目导航A1",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 19,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryNavA1Defaults)
});
