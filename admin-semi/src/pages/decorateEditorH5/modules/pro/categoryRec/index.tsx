/**
 * 分类推荐 模块（契约自 view/admin editor/modules/mobilePro/categoryRec/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultAnimation, defaultModuleStyle, defaultContentStyle, defaultSubTitle } from "../../../contract/sharedDefaults";

/** 演示分类（与旧 categoryRec/index.ts categoryList 逐项一致：6 项，含嵌套 link 与 catPic 全字段） */
const categoryList = [
    {
        title: "小零食",
        link: { link: { link: "", title: "" }, linkTitle: "" },
        titleColor: "",
        backgroundColor: "",
        catPic: {
            picId: 1462,
            picThumb: "https://oss.tigshop.com/img/gallery/202411/1731896143iynI3ymaLzxGaRLk5h.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://oss.tigshop.com/img/gallery/202411/1731896143iynI3ymaLzxGaRLk5h.jpg",
            picName: "FkVzGj1HD2aeDYmJ7E_VaFrIdr0H"
        }
    },
    {
        title: "粮油米面",
        link: { link: { link: "", title: "" }, linkTitle: "" },
        titleColor: "",
        backgroundColor: "",
        catPic: {
            picId: 1465,
            picThumb: "https://oss.tigshop.com/img/gallery/202411/1731896143tV11a96dva46rixHbe.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://oss.tigshop.com/img/gallery/202411/1731896143tV11a96dva46rixHbe.jpg",
            picName: "FmJJUiXFcIwl09Uo9QGJVuiDWhSt"
        }
    },
    {
        title: "洗发水",
        link: { link: { link: "", title: "" }, linkTitle: "" },
        titleColor: "",
        backgroundColor: "",
        catPic: {
            picId: 1464,
            picThumb: "https://oss.tigshop.com/img/gallery/202411/1731896143OEttGMFtuKVqGX0JGF.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://oss.tigshop.com/img/gallery/202411/1731896143OEttGMFtuKVqGX0JGF.jpg",
            picName: "FjJhor6dmekuWG2RdOJkpwDL2JOY"
        }
    },
    {
        title: "肉干",
        link: { link: { link: "", title: "" }, linkTitle: "" },
        titleColor: "",
        backgroundColor: "",
        catPic: {
            picId: 1466,
            picThumb: "https://oss.tigshop.com/img/gallery/202411/17318961436JZDXeYmtceZlmj3GK.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://oss.tigshop.com/img/gallery/202411/17318961436JZDXeYmtceZlmj3GK.jpg",
            picName: "Ftv7Xo3nQ9qZRYPnN9oQB"
        }
    },
    {
        title: "饮料",
        link: { link: { link: "", title: "" }, linkTitle: "" },
        titleColor: "",
        backgroundColor: "",
        catPic: {
            picId: 1463,
            picThumb: "https://oss.tigshop.com/img/gallery/202411/1731896143uW4eqHtyOfGZ1zl7hO.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://oss.tigshop.com/img/gallery/202411/1731896143uW4eqHtyOfGZ1zl7hO.jpg",
            picName: "FgboIgXyCabilx_7jFjqAc2vBrye"
        }
    },
    {
        title: "可爱桌面收纳盒",
        link: { link: { link: "", title: "" }, linkTitle: "" },
        titleColor: "",
        backgroundColor: "",
        catPic: {
            picId: 1461,
            picThumb: "https://oss.tigshop.com/img/gallery/202411/1731896143OQ05zAtqjDwO7YD6jA.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
            picUrl: "https://oss.tigshop.com/img/gallery/202411/1731896143OQ05zAtqjDwO7YD6jA.jpg",
            picName: "FkVNwFY9oMelIYCDC7ZTNLwpeC"
        }
    }
];

/** 与旧 mobilePro/categoryRec/index.ts 逐字段一致（契约） */
export const categoryRecDefaults = {
    categoryList,
    animation: defaultAnimation,
    categoryPadding: 3,
    radius: 25,
    subTitle: defaultSubTitle,
    backgroundColor: "#fff",
    catBorder: 0,
    catBorderWidth: 1,
    catBorderColor: "#ffffff",
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryRec",
    label: "分类推荐",
    icon: '<i class="h5-mod-ico">🗂️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 66,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryRecDefaults)
});
