/**
 * 图文导航 模块（契约自 view/admin editor/modules/mobile/imageNav/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

/**
 * 我方新增默认演示项（2026-11-21）：原版 defaults 的 picList 为空，新增模块在
 * 画布上是空块；按默认网格（rowNum 4 × colNum 2）预置 8 个演示导航项，图标与
 * 标题取分类导航（categoryNav）已迁移演示图标同源（实测 200 + image/png），
 * 新增即成行成列可看，用户可换可删。仅新增模块生效，不回填存量数据。
 */
const demoPicList = [
    { picId: 1377, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947YV2lncqLSOSg8OvpbK.png", picName: "dangrisuda", picTitle: "当日速达" },
    { picId: 1384, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957948LcXRbvCuwlCHyHy7Y0.png", picName: "xiuxianlingshi", picTitle: "休闲零食" },
    { picId: 1378, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947XV6cPNonZJurGcDfOP.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947XV6cPNonZJurGcDfOP.png", picName: "mimianliangyou", picTitle: "粮油米面" },
    { picId: 1385, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17309579481I5gopEn7PkOneF0NU.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17309579481I5gopEn7PkOneF0NU.png", picName: "ruyinjiushui", picTitle: "乳饮酒水" },
    { picId: 1382, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957948Ouz6wPq8cQkgSjDCkF.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957948Ouz6wPq8cQkgSjDCkF.png", picName: "riyongbaihuo", picTitle: "日用百货" },
    { picId: 1380, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947iOLEngIwvzxZDjhtQ6.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947iOLEngIwvzxZDjhtQ6.png", picName: "muyingyongpin", picTitle: "母婴用品" },
    { picId: 1379, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947lWqY7UySCwQ89qsq7f.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957947lWqY7UySCwQ89qsq7f.png", picName: "gerenxihu", picTitle: "个人洗护" },
    { picId: 1383, picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957948vFCfxjt0TIROPOocWI.png?x-oss-process=image/resize,m_pad,h_200,h_200", picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730957948vFCfxjt0TIROPOocWI.png", picName: "zhipinjiaqing", picTitle: "纸品家清" }
];

/** 与 view/admin imageNavDefaults 逐字段一致（契约），仅 picList 为我方新增默认演示项 */
export const imageNavDefaults = {
    navType: 1,
    navStyle: 1,
    rowNum: 4,
    colNum: 2,
    picList: demoPicList,
    imgPadding: 8,
    radioStyle: 1,
    picPageType: 1,
    swiperPageColor: "",
    frame: defaultFrame
};

registerModule({
    type: "imageNav",
    label: "图文导航",
    icon: '<i class="h5-mod-ico">🖼️</i>',
    url: "mobile",
    platform: "mobile",
    order: 9,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(imageNavDefaults)
});
