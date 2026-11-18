/**
 * PC 装修模块 · pc-product-group（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 812–885），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { buyBtnFields, registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-product-group', label: '商品分组', group: '商品展示',
  defaults: () => {
    // 演示商品（原版 demoProductList 逐字段）
    const demoProductList = [
      ['1732004872pw39D1oGqdAaZMpvyo', 88.8, 98.8],
      ['1732004872f6EcZnbAqdW5kIGzxL', 56.8, 68.88],
      ['1732004872o3s8aqPGdICr1DOCdB', 48.8, 69.9],
      ['1732004872Mry8L9BWOCiawPhp72', 48.8, 69.9],
      ['1732004872yPYr4zSj7AXrfw9oON', 32.0, 39.0],
      ['1732004872NlX49KeGeYSN4Tw0wa', 32.0, 39.0],
    ].map(([hash, price, marketPrice]) => ({
      productId: 0,
      productSn: '',
      productName: '商品的大标题雨价格在后台他会自动调用读取',
      productDesc: '',
      productPrice: price as number,
      marketPrice: marketPrice as number,
      picThumb: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.jpg`,
    }));
    const demoGroup = (title: string, withTitlePic: boolean): Record<string, any> => ({
      sourceType: 'products',
      title,
      titlePic: withTitlePic
        ? {
            picUrl: 'https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/07/147580fe25834bb09199267ee601b834.png',
            picThumb: 'https://oss.vortmall.com/demo/upload/c4ca4238/2026/02/07/147580fe25834bb09199267ee601b834.png?x-oss-process=image/resize,m_pad,h_200,h_200',
          }
        : { picUrl: '', picThumb: '' },
      titleLink: { link: '', name: '' },
      bannerPic: { picUrl: '', picThumb: '' },
      bannerLink: { link: '', name: '' },
      productList: demoProductList.map((p) => ({ ...p })),
    });
    return {
      productSource: 'products',
      productGroups: [demoGroup('为你推荐', true), demoGroup('热卖', false), demoGroup('折扣', false), demoGroup('预售', false)],
      groupList: [],
      productCardStyle: 1, listStyle: 4, cardPadding: 0, cardRadius: 4,
      showProductName: 1, productNameRow: 1, productNameWeight: 400, productNameSize: 16,
      showProductPrice: 1, productPriceMarginTop: 10, productPriceWeight: 400, productPriceSize: 28,
      crossedOutPrice: 1, showSales: 1, showShopTag: 1,
      // 购买按钮改全站统一枚举：旧对象型默认 {showBtn:1, btnType:1(加号)} ≡ 新枚举 3（加号描边）
      buyBtnStyle: 3,
      buyBtnIcon: { picUrl: '', picThumb: '' },
      productColor: { infoLeftRightPadding: 10, backgroundColor: '#ffffff', titleColor: '#19191b', priceColor: '', crossedOutPriceColor: '#999999' },
      groupTitle: { marginLeftRight: 5, showTitle: 1, bigTitleFontSize: 14, titleColor: '#444444', activeTitleColor: '#FF0F23', activeTitleBgColor: '#ffebf1', titleMinWidth: 80, titlePaddingTopBottom: 8, titlePaddingLeftRight: 12, titleRadius: 4, splitLineColor: 'transparent', activeBgMarginLeft: 0, activeBgMarginRight: 0 },
      moduleStyle: { boxPaddingTop: 0, boxPaddingBottom: 0, boxPadding: 10, backgroundColor: '' },
      contentStyle: { paddingTop: 0, paddingBottom: 0, paddingLeftRight: 13, boxRadius: 0, backgroundColor: '', backgroundPic: { picUrl: '', picThumb: '' }, backgroundPicFillType: 'cover', boxShadow: 0 },
    };
  },
  settings: [
    { kind: 'groupList', key: 'productGroups', label: '分组列表', hideLabel: true },
    { kind: 'radio', key: 'productCardStyle', label: '商品样式', options: [{ label: '扁平', value: 1 }, { label: '卡片', value: 2 }, { label: '描边', value: 3 }], section: '商品参数设置' },
    { kind: 'radio', key: 'listStyle', label: '展示样式', options: [{ label: '一行四个', value: 4 }, { label: '一行五个', value: 6 }, { label: '一行六个', value: 7 }] },
    { kind: 'slider', key: 'cardPadding', label: '商品内部边距', max: 10 },
    { kind: 'slider', key: 'cardRadius', label: '商品圆角半径', max: 20 },
    { kind: 'radio', key: 'showProductName', label: '商品标题', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    { kind: 'radio', key: 'productNameRow', label: '标题行数', options: [{ label: '一行', value: 1 }, { label: '两行', value: 2 }], showIf: (m) => Number(m.showProductName) === 1 },
    { kind: 'radio', key: 'productNameWeight', label: '标题粗细', options: [{ label: '常规', value: 400 }, { label: '加粗', value: 700 }], showIf: (m) => Number(m.showProductName) === 1 },
    { kind: 'slider', key: 'productNameSize', label: '标题大小', min: 12, max: 16, showIf: (m) => Number(m.showProductName) === 1 },
    { kind: 'radio', key: 'showProductPrice', label: '商品价格', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    { kind: 'radio', key: 'crossedOutPrice', label: '商品划线价', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    { kind: 'radio', key: 'showSales', label: '已售', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    { kind: 'radio', key: 'showShopTag', label: '商品店铺', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    // 购买按钮：原对象型 container（BuyBtnSettings 三样式）改全站统一字段；存量对象数据载入时已归一化
    ...buyBtnFields(),
    { kind: 'container', key: 'productColor', label: '商品颜色设置', containerType: 'productColor' },
    { kind: 'container', key: 'groupTitle', label: '分组标题设置', containerType: 'groupTitle' },
    { kind: 'container', key: 'moduleStyle', label: '模块样式设置', containerType: 'moduleStyle' },
    { kind: 'container', key: 'contentStyle', label: '内容区样式设置', containerType: 'contentStyle' },
  ],
  summary: (m) => `${((m.productGroups || []) as Array<Record<string, any>>).length} 组 · 样式${m.listStyle}`,
};

registerPcModule(def);
export default def;
