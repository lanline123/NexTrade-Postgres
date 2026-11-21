/**
 * 个性商品 模块（契约自 view/admin editor/modules/mobilePro/customProduct/index.ts 逐字段移植）
 *
 * 原版在旧 ToolBar 中已被注释、不进工具栏（visible:false），保留注册仅为老数据
 * 可渲染；且原版两代编辑器 Preview 均为占位文本。本重建已补齐可用闭环——
 * Settings 接入共享 ProductInfoSelect（原版 Settings.vue 同口径，见 Settings 头注）、
 * Preview 按 uniapp-react 渲染端契约真实渲染、渲染端入口已存在
 * （uniapp-react src/pages/customPage/modules/customProduct/index.tsx，早先
 * 「渲染端无入口」的核查结论已被前端审计证伪）——故放开 visible 进工具栏。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultAnimation, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

/**
 * 演示商品（原版 defaults 内置 4 个 productId=0 演示商品，我方补齐 2026-11-21）：
 * 商品名/价格与原版逐字一致，图片为原版同路径素材、宿主换我方 OSS
 * （nlatstore 桶同路径文件已逐个实测 200 + image/jpeg）。
 * 注意（用户口径 2026-11-21）：此处仅为槽位脚手架——模块新建/插入时经
 * 通用填充器 components/demoProductFill 换成商城在售商品随机抽样
 * （取数失败/无在售则移除演示位），编造商品不再写入页面数据；
 * 运行端渲染器亦不展示 productId=0 条目。
 */
const pruductList = [
    {
        productId: 0,
        productSn: "",
        productName: "麦片商品的标题信息",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662JqyhZ6f1BCsWeUJlgj.jpg"
    },
    {
        productId: 0,
        productSn: "",
        productName: "巧克力棒商品的标题信息",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662cOmZ28i8gKl6JiUBad.jpg"
    },
    {
        productId: 0,
        productSn: "",
        productName: "汤达人商品的标题信息",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662AztoZ2OADHp1b8vUoQ.jpg"
    },
    {
        productId: 0,
        productSn: "",
        productName: "士力架商品的标题信息",
        productDesc: "",
        productPrice: 29.0,
        marketPrice: 0,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731908662O7VZIzaFtItS4OvAOR.jpg"
    }
];

/**
 * 侧图：原版预置海报（gallery/202411/1732167975uqaGwJU2aHObkmOmpb.png，picId 1516）
 * 未随素材迁移进我方 OSS（同路径实测 404），以同批已迁移的演示图
 * （商品混排演示图，picId 1454，实测 200）替代；对象形态与原版一致。
 */
const url = {
    picId: 1454,
    picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293Kfvp2BXElAzXCOY7pq.png?x-oss-process=image/resize,m_pad,h_200,h_200",
    picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1731650293Kfvp2BXElAzXCOY7pq.png",
    picName: "Fi_ZwwrzwDQddtmqTBCAyht4dPI"
};

/** 字段与旧 mobilePro/customProduct/index.ts 逐字段一致（契约） */
export const customProductDefaults = {
    pruductList: pruductList,
    picUrl: url,
    picLocation: "right",
    animation: defaultAnimation,
    picFillType: "cover",
    productRadius: 5,
    picBottomRadius: 1,
    crossedOutPrice: 1,
    moduleColor: {
        productBackgroundColor: "#ffffff",
        productNameColor: "#333333",
        productPriceColor: "#72b76c",
        crossedOutPriceColor: "#999999"
    },
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "customProduct",
    label: "个性商品",
    icon: '<i class="h5-mod-ico">🛒</i>',
    url: "mobilePro",
    platform: "mobile",
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(customProductDefaults)
});
