/**
 * 通用标题 模块（渲染端 registry 独有 type，编辑器新注册；
 * 默认值字段对齐 uniapp-react 渲染端 customPage/modules/commonTitle/index.tsx 读取的 module 字段）
 *
 * 注意：本模块的数据本身即标题对象（渲染端直接读 module.showTitle 等），
 * 与其他模块嵌套在 module.title 下不同。渲染端读取：showTitle、titleText、descText、
 * titleColor、descColor、titleStyle、titleBackground、titleBackground2、titleRadius、
 * titleBackgroundPic{picUrl,picThumb}、showMore、moreColor、moreLink。
 * 字段取值与 contract/sharedDefaults 的 defaultTitle 对应字段一致。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

/** 与渲染端 commonTitle/index.tsx 读取字段逐项对应（取值对齐 defaultTitle） */
export const commonTitleDefaults = {
    showTitle: 1,
    titleText: "标题内容",
    descText: "",
    titleColor: "",
    descColor: "#aaaaaa",
    titleStyle: 1,
    titleBackground: "",
    titleBackground2: "",
    titleRadius: 0,
    titleBackgroundPic: {
        picUrl: "",
        picThumb: ""
    },
    showMore: 0,
    moreColor: "#aaaaaa",
    moreLink: [] as Array<Record<string, any>>
};

registerModule({
    type: "commonTitle",
    label: "通用标题",
    icon: '<i class="h5-mod-ico">🏷️</i>',
    url: "mobile",
    platform: "mobile",
    order: 18,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(commonTitleDefaults)
});
