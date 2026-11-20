/**
 * 图片魔方 模块（契约自 view/admin editor/modules/mobile/imageSquareAd/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame, defaultTitle } from "../../../contract/sharedDefaults";

/**
 * 我方新增默认演示图（2026-11-21）：原版 defaults 的 picList 为空，新增模块在
 * 画布上是空块；默认模板 picType=1（一行两个），预置 2 张已迁移演示图
 * （与首焦轮播演示图同源，实测 200），新增即有图可看，用户可换可删。
 * 仅新增模块生效，不回填存量数据。
 */
const demoPicList = [
    {
        picId: 1414,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731044101tXK31aimefqoUsJSrM.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731301937REbUnsSmPehWiBMaFd.webp",
        picName: "FjfhKDiUzqW_lqfJqncKt3QJplx4"
    },
    {
        picId: 1415,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731044101j8COoqtfDJCWrSts6b.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731301941uaSjIShOBNL2VBSr0y.webp",
        picName: "Fv5iP2IHByVFUlGrLkXtPkClndla"
    }
];

/** 与 view/admin imageSquareAdDefaults 逐字段一致（契约），仅 picList 为我方新增默认演示图 */
export const imageSquareAdDefaults = {
    picType: 1,
    picList: demoPicList,
    swiperPreView: 1,
    swiperPageColor: "",
    imgPadding: 0,
    picPageType: 1,
    picRadioStyle: 1,
    isFluxWidth: 0,
    frame: defaultFrame,
    title: defaultTitle
};

registerModule({
    type: "imageSquareAd",
    label: "图片魔方",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobile",
    platform: "mobile",
    order: 3,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(imageSquareAdDefaults)
});
