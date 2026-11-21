/**
 * 贴附标题条 模块（原显示名「通用标题」，2026-11-21 用户拍板改名区分定位；
 * 内部 type 仍为 commonTitle、数据结构不变。渲染端 registry 独有 type，编辑器新注册；
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

/**
 * 与渲染端 commonTitle/index.tsx 读取字段逐项对应（取值对齐 defaultTitle）；
 * 另补齐 defaultTitle 的其余 5 个字段（titleAlign/titleWeight/moreType/countdownLabel/format），
 * 使独立模块数据结构与原版 title 对象 18 字段一致（渲染端不读这 5 个，不影响渲染）。
 * showTitle 例外：原版 defaultTitle 为 0（嵌套标题对象口径），独立模块渲染端
 * showTitle != 1 直接不渲染，新建默认必须为 1，否则加进画布即不可见。
 */
export const commonTitleDefaults = {
    showTitle: 1,
    titleText: "标题内容",
    descText: "",
    titleColor: "",
    descColor: "#aaaaaa",
    titleStyle: 1,
    titleAlign: 1,
    titleWeight: "normal",
    titleBackground: "",
    titleBackground2: "",
    titleRadius: 0,
    titleBackgroundPic: {
        picUrl: "",
        picThumb: ""
    },
    showMore: 0,
    moreType: 1,
    countdownLabel: "",
    moreColor: "#aaaaaa",
    moreLink: [] as Array<Record<string, any>>,
    format: {
        titleBackground: "",
        titleRadius: ""
    }
};

/** 模块说明（模块库悬停与设置面板提示共用一句：与「文本标题」的区别） */
export const commonTitleDesc =
    "贴附式标题条：紧贴下方模块的帽子条，背景色/渐变/小图标刷在标题条本身、只圆上两角；需要独立成段的白底标题块请用「文本标题」";

registerModule({
    type: "commonTitle",
    label: "贴附标题条",
    desc: commonTitleDesc,
    icon: '<i class="h5-mod-ico">🏷️</i>',
    url: "mobile",
    platform: "mobile",
    order: 18,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(commonTitleDefaults)
});
