/**
 * 秒杀（样式2） 模块（默认值字段对齐 uniapp-react 渲染端 customPage/modules/seckill2/index.tsx 读取的 module 字段）
 *
 * 渲染端读取：frame、title（defaultTitle 结构，标题右侧挂「距结束」倒计时）、style
 * （10=横向滑动三列）、buyBtnStyle、goodsStyle、goodsRadioStyle、textAlign、textWeight、
 * goodsNameRow、goodsNamePadding、showName、showPrice、imageBackgroundColor、
 * backgroundColor、boxPadding、boxPaddingTop、boxPaddingBottom、boxRadius、innerPadding、
 * goodsPadding；商品由渲染端接口 getHomeSeckill 拉取。
 * products 为编辑器侧商品来源配置（与商品模块同构，保留契约字段）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame, defaultTitle, defaultProducts } from "../../../contract/sharedDefaults";

/** 与渲染端 seckill2/index.tsx 读取字段逐项对应 */
export const seckill2Defaults = {
    style: 10,
    goodsStyle: 1,
    goodsRadioStyle: 1,
    textAlign: 1,
    textWeight: 1,
    goodsNameRow: 2,
    goodsNamePadding: 1,
    showName: 1,
    showPrice: 1,
    buyBtnStyle: 1,
    backgroundColor: "",
    imageBackgroundColor: "",
    boxRadius: 0,
    innerPadding: 0,
    boxPadding: 10,
    boxPaddingTop: 5,
    boxPaddingBottom: 5,
    goodsPadding: 5,
    frame: defaultFrame,
    title: defaultTitle,
    products: defaultProducts
};

registerModule({
    type: "seckill2",
    label: "秒杀（样式2）",
    icon: '<i class="h5-mod-ico">⏱️</i>',
    url: "mobilePromotion",
    platform: "mobile",
    order: 34,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(seckill2Defaults)
});
