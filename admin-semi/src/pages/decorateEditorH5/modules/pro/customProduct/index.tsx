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
 * 字段与旧 mobilePro/customProduct/index.ts 逐字段一致（契约）；
 * pruductList / picUrl 的值取空（原版 defaults 内置的 4 个演示商品与预置海报
 * 均为 tigshop OSS 素材，不随重建引入，见交付报告差异点）。
 */
export const customProductDefaults = {
    pruductList: [],
    picUrl: { picUrl: "", picThumb: "" },
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
