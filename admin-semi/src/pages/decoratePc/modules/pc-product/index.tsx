/**
 * PC 装修模块 · pc-product（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 784–811），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { buyBtnFields, containerFields, productsDefaults, registerPcModule, standardFrame, standardTitle } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-product', label: '商品', group: '商品展示',
  defaults: () => ({
    style: 1, goodsStyle: 1, goodsRadioStyle: 1, textAlign: 1, textWeight: 1, goodsNameRow: 2,
    goodsNamePadding: 1, showName: 1, showBrief: 1, showPrice: 1, goodsPadding: 5, buyBtnStyle: 1,
    buyBtnIcon: { picUrl: '', picThumb: '' },
    backgroundColor: '', boxRadius: 0, innerPadding: 0, boxPadding: 10, boxPaddingTop: 5,
    boxPaddingBottom: 5, imgPadding: 1, waterfall: 0, swiperPageColor: '',
    frame: standardFrame(),
    title: standardTitle(),
    products: productsDefaults(),
  }),
  settings: [
    { kind: 'radio', key: 'style', label: '展示样式', options: [{ label: '一行四个', value: 1 }, { label: '一行五个', value: 2 }] },
    { kind: 'slider', key: 'goodsPadding', label: '商品边距', max: 30 },
    // 原版 PC 商品面板未暴露这三项与购买按钮样式（默认值写死在 index.ts），
    // 此处补齐，使模块行为可配置又不改变默认表现（默认值与原版一致：全部显示 / 购物车图标）。
    { kind: 'switch', key: 'showName', label: '显示商品名称', section: '商品内容' },
    { kind: 'switch', key: 'showBrief', label: '显示商品简介', section: '商品内容' },
    { kind: 'switch', key: 'showPrice', label: '显示商品价格', section: '商品内容' },
    // 购买按钮：全站统一字段（枚举 0–10 + 跟随全局 + 自定义图标），与前台共享组件同口径
    ...buyBtnFields(),
    { kind: 'products', key: 'products', label: '选择商品', section: '商品设置' },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    ...containerFields({ showInnerPadding: true }),
  ],
  summary: (m) => `样式${Number(m.style) === 2 ? 5 : 4}列 · ${((m.products?.productIds || []) as number[]).length} 商品`,
};

registerPcModule(def);
export default def;
