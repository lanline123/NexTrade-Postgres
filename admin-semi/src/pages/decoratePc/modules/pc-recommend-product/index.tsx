/**
 * PC 装修模块 · pc-recommend-product（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 886–927），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { containerFields, getPath, productsDefaults, registerPcModule, standardTitle } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-recommend-product', label: '推荐商品', group: '商品',
  // 2026-11-12 用户指令：模块选单里要能看到推荐商品（此前 hiddenInLibrary: true 导致它只存在于老页面数据中，选不出来）。
  defaults: () => ({
    // 模块背景默认透明（2026-11-13 用户指定）：页面底色的浅灰要透出来，模块自身不铺白
    frame: { boxPadding: 10, boxPaddingTop: 10, boxPaddingBottom: 50, boxRadius: 0, backgroundColor: 'transparent', itemGap: 16, columns: 5 },
    titleName: '猜你喜欢',
    // 标题设置块（与 pc-product 同款）。老数据只有 titleName、没有 title 键：
    // 预览/前台按「data 里有 title 键才走标题块，否则回退 titleName」判断，避免双重渲染。
    // 2026-11-13 用户指定新建默认值：标题样式1 / 居中 / 24 号字。
    title: standardTitle({ showTitle: 1, titleText: '猜你喜欢', titleColor: '#2a3145', titleStyle: 1, titleAlign: 2, titleFontSize: 24 }),
    showProductName: 1,
    showProductPrice: 1,
    products: {
      ...productsDefaults(),
      productNumber: 20,
      // 智能推荐（千人千面）：前台按访客行为（购物车/浏览/已购/收藏）实时算偏好分类并排序；
      // 默认关闭，老页面模块不受影响，开启后「选择商品」的取数方式让位于算法（无行为时热门兜底）。
      algorithm: { enabled: 0, useCart: 1, useHistory: 1, usePurchased: 1, useFavorite: 1, useHot: 1, useCohort: 1, defaultRows: 2, maxRows: 10, infiniteScroll: 0 },
    },
  }),
  settings: [
    { kind: 'radio', key: 'frame.columns', label: '展示样式', extend: false, options: [{ label: '一行三个', value: 3 }, { label: '一行四个', value: 4 }, { label: '一行五个', value: 5 }, { label: '一行六个', value: 6 }], section: '商品参数设置' },
    { kind: 'slider', key: 'frame.itemGap', label: '商品间距', extend: false, max: 40, section: '商品参数设置' },
    { kind: 'radio', key: 'showProductName', label: '显示商品名称', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }], section: '商品参数设置' },
    { kind: 'radio', key: 'showProductPrice', label: '显示商品价格', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }], section: '商品参数设置' },
    { kind: 'products', key: 'products', label: '选择商品', section: '商品设置', numberMax: 50 },
    { kind: 'switch', key: 'products.algorithm.enabled', label: '智能推荐（千人千面）', section: '算法设置', hint: '开启后，前台按每位访客的购物车、浏览、已购、收藏行为实时推荐相关商品；未登录或无行为记录时展示热门商品。「选择商品」的取数设置在开启时不生效。' },
    { kind: 'switch', key: 'products.algorithm.useCart', label: '参考购物车商品', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1 },
    { kind: 'switch', key: 'products.algorithm.useHistory', label: '参考浏览记录', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1 },
    { kind: 'switch', key: 'products.algorithm.usePurchased', label: '参考已购商品', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1 },
    { kind: 'switch', key: 'products.algorithm.useFavorite', label: '参考收藏商品', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1 },
    { kind: 'switch', key: 'products.algorithm.useHot', label: '参考热销商品', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1, hint: '全站销量最高的商品会掺入推荐流（排在与访客兴趣相关的商品之后）。' },
    { kind: 'switch', key: 'products.algorithm.useCohort', label: '参考相似用户购买', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1, hint: '与该访客买过相同分类的其他用户近期购买的商品（协同过滤，需后端新版接口 cohortBuys 支持；后端未发版前此来源自动跳过，不影响其他来源）。' },
    { kind: 'number', key: 'products.algorithm.defaultRows', label: '默认显示行数', min: 1, max: 20, section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1, hint: '首屏展示的行数（行数 × 每行列数 = 商品数），继续下拉按此行数逐段加载。' },
    { kind: 'number', key: 'products.algorithm.maxRows', label: '最多显示行数', min: 1, max: 100, section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1 && Number(getPath(d, 'products.algorithm.infiniteScroll')) !== 1, hint: '下拉加载到这个行数就停；不足时以实际商品数为准。' },
    { kind: 'switch', key: 'products.algorithm.infiniteScroll', label: '无限下拉加载', section: '算法设置', showIf: (d) => Number(getPath(d, 'products.algorithm.enabled')) === 1, hint: '开启后不限制行数，一直下拉直至全部可推荐商品加载完。' },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    ...containerFields(),
  ],
  summary: (m) => `${getPath(m, 'title.titleText') || m.titleName || ''} · 一行${Number(getPath(m, 'frame.columns')) || 5}列 · ${((m.products?.productIds || []) as number[]).length} 商品`,
};

registerPcModule(def);
export default def;
