import { buyBtnFields, registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

/** 搜索结果模块（pageType=6 搜索页专属，对齐原版 PcSearchResult）。
 *  数据契约与 PC 前台 app/search/page.tsx 的 PcSearchConfig 一一对应：
 *  前台只认 type=pcSearchResult 且 url 为空或 'pcSearch' 的第一个模块，
 *  开关字段经 normalizeBoolLike 归一化（true/1 均可），编辑器 switch 写 1/0 安全。 */
export const PC_SEARCH_RESULT_DEFAULTS = {
  backgroundColor: '#ffffff',
  themeColor: '#ff4d4f',
  filterStyle: 2,
  filterTemplate: 2,
  goodsStyle: 1,
  goodsRadiusStyle: 1,
  // 购买按钮改全站统一枚举：新默认 5（"购买"描边）≡ 旧默认 buyStyle=2 的观感；buyStyleV=2 标记新格式
  buyStyle: 5,
  buyStyleV: 2,
  buyBtnIcon: { picUrl: '', picThumb: '' } as { picUrl: string; picThumb: string } | null,
  showFilterPanel: true as boolean | number,
  showPriceRange: true as boolean | number,
  showSales: false as boolean | number,
  showShopTag: false as boolean | number,
  showCartButton: true as boolean | number,
  componentMarginTop: 24,
  componentMarginBottom: 24,
};

const PC_SEARCH_RESULT_MODULE_DEF: ModuleDef = {
  type: 'pcSearchResult',
  label: '搜索结果',
  group: 'pcSearchResult',
  hiddenInLibrary: false,
  defaults: () => ({ ...PC_SEARCH_RESULT_DEFAULTS }),
  settings: [
    { kind: 'radio', key: 'filterStyle', label: '筛选布局', options: [{ label: '侧栏筛选', value: 1 }, { label: '顶部筛选', value: 2 }], section: '筛选设置' },
    { kind: 'radio', key: 'filterTemplate', label: '商品列数', options: [{ label: '一行五列', value: 1 }, { label: '一行六列', value: 2 }], section: '筛选设置' },
    { kind: 'switch', key: 'showFilterPanel', label: '显示筛选面板', section: '筛选设置' },
    { kind: 'switch', key: 'showPriceRange', label: '显示价格区间', section: '筛选设置' },
    { kind: 'radio', key: 'goodsStyle', label: '商品卡片边框', options: [{ label: '无边框', value: 1 }, { label: '卡片', value: 2 }, { label: '描边', value: 3 }, { label: '投影', value: 4 }], section: '商品设置' },
    { kind: 'radio', key: 'goodsRadiusStyle', label: '商品圆角', options: [{ label: '直角', value: 1 }, { label: '圆角', value: 2 }], section: '商品设置' },
    // 购买按钮：旧「样式一/二/三」改全站统一字段（buyStyle 存统一枚举，buyStyleV=2 区分存量旧值）
    ...buyBtnFields({ key: 'buyStyle', section: '商品设置' }),
    { kind: 'switch', key: 'showCartButton', label: '显示购物车按钮', section: '商品设置' },
    { kind: 'switch', key: 'showSales', label: '显示销量', section: '商品设置' },
    { kind: 'switch', key: 'showShopTag', label: '显示店铺标签', section: '商品设置' },
    { kind: 'color', key: 'backgroundColor', label: '背景颜色', section: '样式设置', resetValue: '#ffffff' },
    { kind: 'color', key: 'themeColor', label: '主题色（选中态）', section: '样式设置', resetValue: '#ff4d4f' },
    { kind: 'slider', key: 'componentMarginTop', label: '上边距', max: 60, section: '样式设置' },
    { kind: 'slider', key: 'componentMarginBottom', label: '下边距', max: 60, section: '样式设置' },
  ],
  summary: (m) => `搜索结果 · ${Number(m?.filterTemplate) === 2 ? '六列' : '五列'} · ${Number(m?.filterStyle) === 1 ? '侧栏筛选' : '顶部筛选'}`,
};

registerPcModule(PC_SEARCH_RESULT_MODULE_DEF);
export default PC_SEARCH_RESULT_MODULE_DEF;
