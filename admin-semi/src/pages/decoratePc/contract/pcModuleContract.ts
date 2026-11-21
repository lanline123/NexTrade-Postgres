/**
 * PC 装修编辑器 · 模块契约（Step 3 模块化基础设施，合并线统一维护，各拆分线勿改）
 * 内容逐字摘自 PcDecorateEditor.tsx：PicItem/演示数据/standardFrame/standardTitle/
 * productsDefaults/SettingFieldDef/ModuleDef/字段工厂（containerFields/buyBtnFields/titleFields）。
 * 注册表：各模块目录 index.tsx 调 registerPcModule 自注册，合并线接线消费。
 */
import i18nInstance from '../../../i18n';

export const tl = (key: string, params?: Record<string, string | number>) =>
  (i18nInstance as any).t(key, params) as string;

export interface PicItem {
  picId?: string;
  picUrl: string;
  picThumb?: string;
  picName?: string;
  picTitle?: string;
  picDesc?: string;
  /** 分类导航3 卡片级图片背景色（原版 PcCategoryNav3Item） */
  imageBackgroundColor?: string;
  picLink?: { link?: string; name?: string; title?: string };
}

export const DEMO_PIC_1 = 'https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/1731301937REbUnsSmPehWiBMaFd.webp';
export const DEMO_PIC_2 = 'https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/1731301941uaSjIShOBNL2VBSr0y.webp';
export const NOTICE_ICON = 'https://oss.vortmall.com/upload/c4ca4238/img/gallery/202508/1754895669bP3iJSUiXSBoRB1VQq.png';
export const demoPic = (id: string, url: string, name: string, title: string): PicItem => ({
  picId: id,
  picUrl: url,
  picThumb: url,
  picName: name,
  picTitle: title,
  picLink: { link: '', name: '' },
});

export const standardFrame = (extra: Record<string, unknown> = {}) => ({
  textColor: '',
  itemBackgroundColor: '',
  backgroundColor: '#ffffff',
  innerPadding: 0,
  itemHeight: 35,
  itemRadius: 3,
  boxRadius: 0,
  boxPadding: 10,
  leftAndRightPadding: 0,
  boxPaddingTop: 0,
  boxPaddingBottom: 0,
  ...extra,
});

export const standardTitle = (extra: Record<string, unknown> = {}) => ({
  showTitle: 0,
  titleStyle: 1,
  titleAlign: 1,
  // 主/副标题字号与描述排版：前台 ShopTitle 与编辑器共享 ModuleTitlePreview 同步消费。
  // 默认值 = 前台现状（18px / 13px / 跟随主标题后），老数据无字段时不产生视觉跳变。
  titleFontSize: 18,
  descFontSize: 13,
  descLayout: 1,
  titleBackground: '',
  titleBackground2: '',
  titleBackgroundPic: { picUrl: '', picThumb: '' },
  titleRadius: 0,
  titlePaddingTop: 15,
  titlePaddingBottom: 15,
  // 标题左右边距：默认 10 = 旧 CSS（.dec-module-title padding:10px）的现状值，老数据零视觉跳变；
  // 设 0 即紧贴模块最左/最右（此时再配合「容器设置 → 左右边距」为 0 就是完全贴边）。
  titlePaddingLeft: 10,
  titlePaddingRight: 10,
  titleText: '标题内容',
  titleColor: '',
  descText: '',
  descColor: '#aaaaaa',
  showMore: 0,
  moreType: 1,
  countdownLabel: '',
  moreLink: { link: '', title: '' },
  moreColor: '#000000',
  ...extra,
});

/** 固定标题改配置型标题之前，线上数据把标题字段平铺在 module 上（优惠券的 title 还是字符串）。
 *  载入时搬进 title 契约，已有 title 对象优先，避免老页面文案丢失或面板显示「不显示标题」。 */
export const normalizeLegacyTitle = (type: string, m: Record<string, any>): Record<string, any> => {
  if (!m || typeof m !== 'object') return m;
  let legacy: Record<string, unknown> | null = null;
  if (type === 'pc-category-nav') {
    legacy = { showTitle: m.showTitle ?? 1, titleText: m.titleName };
  } else if (type === 'pc-category-nav3') {
    legacy = {
      showTitle: 1,
      titleText: m.titleText,
      showMore: m.showMore ?? 1,
      moreLink: { link: m.moreLink?.link || '', title: m.moreLink?.name || m.showMoreText || '更多' },
    };
  } else if (type === 'pc-home-coupon') {
    legacy = {
      showTitle: m.showTitle ?? 0,
      titleText: typeof m.title === 'string' ? m.title : '',
      descText: m.desc,
      titleFontSize: m.titleFontSize,
      descFontSize: m.descFontSize,
      titleAlign: m.titleAlign,
      descLayout: m.descLayout,
    };
  } else if (type === 'pc-home-seckill') {
    legacy = { titleText: m.seckillTitle };
  } else if (type === 'pc-recommend-product') {
    // 老数据只有 titleName、没有 title 对象：搬进共享契约（有文案才让标题栏出现）。
    // 与 pc-semi 的 normalizeLegacyTitle 同一口径，双端不再各自回退。
    const legacyName = typeof m.titleName === 'string' ? m.titleName.trim() : '';
    legacy = legacyName ? { showTitle: 1, titleText: legacyName } : null;
  }
  if (!legacy) return m;
  const defined = Object.fromEntries(Object.entries(legacy).filter(([, v]) => v !== undefined && v !== null && v !== ''));
  const current = m.title && typeof m.title === 'object' ? m.title : {};
  return { ...m, title: { ...standardTitle(), ...defined, ...current } };
};

/** 购买按钮存量数据归一化（载入时在工作副本内完成，保存时自然写新格式；不批量改库）。
 *  与 pc-semi 前台 resolveBuyBtnStyle 同口径：
 *  - pc-product-group 对象型 {showBtn, btnType} → 数字统一枚举（showBtn 关 → 0 隐藏；
 *    btnType 1→3 加号描边、2→1 购物车描边、3→9 圆圈购物车），对象上的自定义图标搬到 buyBtnIcon；
 *    旧对象型的 text（自定义文字）从未被任何渲染消费，一并丢弃；
 *  - 秒杀/拼团（含旧版 pcGroupon）：无 buyBtnSet 标记的存量数据里 0/缺省表示「未配置」
 *    （旧编辑器无此设置、前台恒显示加号），归一化为显式 4（加号实心）+ buyBtnSet=1，
 *    之后 0 才表示隐藏、-1 表示跟随全局；
 *  - 搜索结果 pcSearchResult：buyStyleV<2 时旧 buyStyle 1/2/3 → 统一枚举 2/5/6 并打 buyStyleV=2。 */
export const normalizeLegacyBuyBtn = (type: string, m: Record<string, any>): Record<string, any> => {
  if (!m || typeof m !== 'object') return m;
  if (type === 'pc-product-group') {
    const raw = m.buyBtnStyle;
    if (raw && typeof raw === 'object') {
      const mapped = ({ 1: 3, 2: 1, 3: 9 } as Record<number, number>)[Number(raw.btnType)] ?? 1;
      const style = raw.showBtn ? mapped : 0;
      const icon = raw.iconImage ?? raw.customIcon;
      return {
        ...m,
        buyBtnStyle: style,
        ...(icon && typeof icon === 'object' ? { buyBtnIcon: icon } : {}),
      };
    }
    if (raw === undefined || raw === null) return { ...m, buyBtnStyle: -1 };
    return m;
  }
  if (type === 'pc-home-seckill' || type === 'pc-home-groupon' || type === 'pcGroupon') {
    if (!m.buyBtnSet) {
      const n = Number(m.buyBtnStyle);
      const style = m.buyBtnStyle === undefined || m.buyBtnStyle === null || !Number.isFinite(n) || n === 0 ? 4 : n;
      return { ...m, buyBtnStyle: style, buyBtnSet: 1 };
    }
    return m;
  }
  if (type === 'pcSearchResult') {
    if (!(Number(m.buyStyleV) >= 2)) {
      const legacy = ({ 1: 2, 2: 5, 3: 6 } as Record<number, number>)[Number(m.buyStyle)] ?? 2;
      return { ...m, buyStyle: legacy, buyStyleV: 2 };
    }
    return m;
  }
  return m;
};

export const productsDefaults = () => ({
  productSelectType: 1,
  productIds: [] as number[],
  productCategoryId: undefined as unknown as number,
  productTag: '',
  productNumber: 10,
});

/** 快捷导航演示项（原版 defaultItems 逐字段） */
export const QUICK_NAV_ITEMS: PicItem[] = [
  ['1377', '1730957947YV2lncqLSOSg8OvpbK', '当日速达'],
  ['1384', '1730957948LcXRbvCuwlCHyHy7Y0', '休闲零食'],
  ['1378', '1730957947XV6cPNonZJurGcDfOP', '粮油米面'],
  ['1385', '17309579481I5gopEn7PkOneF0NU', '乳饮酒水'],
  ['1382', '1730957948Ouz6wPq8cQkgSjDCkF', '日用百货'],
  ['1380', '1730957947iOLEngIwvzxZDjhtQ6', '母婴用品'],
  ['1379', '1730957947lWqY7UySCwQ89qsq7f', '个人洗护'],
  ['1383', '1730957948vFCfxjt0TIROPOocWI', '纸品家清'],
  ['1376', '1730957947BswaPOfoOZnn8tDSX6', '护肤美容'],
  ['1381', '1730957947vIKQibO98m2q9oP8ct', '方便速食'],
].map(([picId, hash, picTitle]) => ({
  picId,
  picThumb: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.png?x-oss-process=image/resize,m_pad,h_200,h_200`,
  picUrl: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.png`,
  picName: picId,
  picTitle,
  picLink: { link: '', name: '' },
}));

export const HERO_CATEGORIES: PicItem[] = [
  ['1377', '1730957947YV2lncqLSOSg8OvpbK', '女装 / 男装 / 内衣'],
  ['1384', '1730957948LcXRbvCuwlCHyHy7Y0', '鞋包 / 女靴 / 男鞋'],
  ['1378', '1730957947XV6cPNonZJurGcDfOP', '美妆 / 养护 / 美容'],
  ['1385', '17309579481I5gopEn7PkOneF0NU', '冰洗 / 空调 / 电视 / 厨卫大件'],
  ['1382', '1730957948Ouz6wPq8cQkgSjDCkF', '电脑 / 办公 / 文具用品'],
  ['1380', '1730957947iOLEngIwvzxZDjhtQ6', '食品 / 酒类 / 生鲜 / 特产'],
  ['1379', '1730957947lWqY7UySCwQ89qsq7f', '生活电器 / 厨房小电 / 个护健康'],
  ['1383', '1730957948vFCfxjt0TIROPOocWI', '医药 / 器械 / 保健品'],
  ['1376', '1730957947BswaPOfoOZnn8tDSX6', '配饰 / 手表 / 玉石'],
  ['1381', '1730957947vIKQibO98m2q9oP8ct', '家装 / 厨卫 / 家具'],
  ['1386', '1730957947YV2lncqLSOSg8OvpbK', '汽车 / 摩托 / 配件'],
].map(([picId, hash, picTitle]) => ({
  picId,
  picThumb: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.png?x-oss-process=image/resize,m_pad,h_200,h_200`,
  picUrl: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.png`,
  picName: picId,
  picTitle,
  picDesc: '',
  picLink: { link: '', name: '' },
}));

export const HERO_DEMO_PRODUCTS = [
  '1732004872pw39D1oGqdAaZMpvyo',
  '1732004872f6EcZnbAqdW5kIGzxL',
  '1732004872o3s8aqPGdICr1DOCdB',
  '1732004872Mry8L9BWOCiawPhp72',
  '1732004872yPYr4zSj7AXrfw9oON',
  '1732004872NlX49KeGeYSN4Tw0wa',
].map((hash, i) => ({
  pic: {
    picUrl: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.jpg`,
    picThumb: `https://oss.vortmall.com/upload/c4ca4238/img/gallery/202411/${hash}.jpg`,
  },
  price: ['¥88.8', '¥56.8', '¥48.8', '¥48.8', '¥32.0', '¥32.0'][i],
}));

export const heroDemoModules = (): Array<Record<string, any>> => {
  const defs: Array<[string, string, string, string]> = [
    ['限时秒杀', '钜惠开秒', '#FF4E4E', '#E1251B'],
    ['商家严选', '品质好物', '#FF4E4E', '#E1251B'],
    ['严选臻品', '优中选优', '#6C8CFF', '#3A5BFF'],
    ['新品上市', '重磅上新', '#00C176', '#008856'],
    ['领券中心', '券', '#FF4E4E', '#E1251B'],
    ['组团拼单', '拼享低价', '#FF9000', '#FF5000'],
  ];
  const picks: number[][] = [[2, 3], [4, 5], [0, 1], [2, 3], [4, 5], [0, 1]];
  return defs.map(([moduleTitle, moduleSubTitle, a, b], i) => ({
    moduleTitle,
    moduleSubTitle,
    moduleSubTitleBgColorA: a,
    moduleSubTitleBgColorB: b,
    moduleSubTitleBgType: 'leftRight',
    showPrice: 1,
    link: { link: '', name: '' },
    products: picks[i].map((p) => ({ ...HERO_DEMO_PRODUCTS[p] })),
  }));
};

/* ===================== 设置字段定义 ===================== */

export interface SettingFieldDef {
  kind:
    | 'radio' | 'switch' | 'select' | 'input' | 'textarea' | 'number' | 'slider'
    | 'color' | 'colorPair' | 'squareTemplate' | 'picList' | 'pic' | 'link' | 'linkList'
    | 'category' | 'categoryMulti' | 'products' | 'productIds' | 'groupon'
    | 'title' | 'container' | 'hotarea' | 'groupList' | 'heroCards' | 'buyBtnStyle';
  key: string;
  label: string;
  hint?: string;
  /** 依当前模块数据动态计算的提示文案（优先级高于 hint，如 image-ad 随 picType 变化的建议尺寸） */
  hintFor?: (m: Record<string, any>) => string;
  /** 输入框占位文案（对齐旧版 Settings.vue 各 el-input 的 placeholder） */
  placeholder?: string;
  rows?: number;
  defaultValue?: number;
  options?: Array<{ label: string; value: number | string }>;
  max?: number;
  min?: number;
  numberMin?: number;
  numberMax?: number;
  numberStep?: number;
  /** container 字段的子结构类型 */
  containerType?: 'frame' | 'buyBtn' | 'productColor' | 'groupTitle' | 'moduleStyle' | 'contentStyle';
  /** color 字段"重置"按钮写回的值（原版各 SelectColor 的 dec-color-reset 语义），缺省为 '' */
  resetValue?: unknown;
  /** title 字段是否隐藏"更多链接/倒计时"二选（原版 enableMoreType 未开启的模块） */
  titleHideMoreType?: boolean;
  /** title 字段是否隐藏标题上/下内边距（原版 enableTitleVerticalPadding 未开启的模块） */
  titleHideVerticalPadding?: boolean;
  /** picList 字段是否显示描述输入（原版 ModulePicList show-desc） */
  picDesc?: boolean;
  /** picList 添加按钮文案（原版 ModulePicList title） */
  picAddText?: string;
  /** picList 动态添加按钮文案（图文导航随 navStyle 变化） */
  picAddTextFor?: (m: Record<string, any>) => string;
  /** picList 是否隐藏链接（原版 ModulePicList show-link=false） */
  picNoLink?: boolean;
  /** picList 是否隐藏标题输入（原版 ModulePicList show-text=false） */
  picNoText?: boolean;
  /** 该字段属于原版「扩展设置」而非「内容设置」 */
  extend?: boolean;
  /** 字段前的分组标题（原版 dec-edit-title-box） */
  section?: string;
  /** 不渲染字段级分组标题（如原版 pc-product-group 的「购买按钮」直接以行 label 呈现） */
  hideSection?: boolean;
  /** 不渲染字段行 label（如 pc-product-group 的分组列表：内容本身就是完整块，无需再占一行标题） */
  hideLabel?: boolean;
  /** 同一字段前连续多个分组标题（如营销通栏 中间焦点区 → 轮播广告） */
  sections?: string[];
  /** 图片列表最大数量（原版 ModulePicList max-count） */
  picMax?: number;
  /** 依模块数据动态计算图片最大数量（如 pc-image-square-ad 随布局固定张数） */
  picMaxFor?: (m: Record<string, any>) => number;
  /** colorPair 的第二个颜色 key */
  secondKey?: string;
  /** colorPair 第二个颜色的重置值 */
  resetValueSecond?: unknown;
  /** 字段写入后的附加行为（如切换拼团类型时清空活动） */
  onChangeExtra?: (moduleData: Record<string, any>, newValue: unknown) => Record<string, any>;
  showIf?: (m: Record<string, any>) => boolean;
  /**
   * radio 字段渲染成原版的「图标按钮组」：每个选项显示 `ico-decorate` 字形并带 tooltip 文案。
   * 原版 el-radio-button + el-tooltip 的形态（如 选择模板 / 指示器样式）。
   */
  iconRadio?: boolean;
  /** 选项对应的装饰图标 code point（配合 iconRadio，与 options 等长或按 value 映射） */
  optionIcons?: Record<string | number, string>;
  /** kind='buyBtnStyle' 时模块自带自定义图标的字段 key（样式 10 的选择器预览图；缺省 buyBtnIcon） */
  buyBtnIconKey?: string;
}

export interface ModuleDef {
  type: string;
  label: string;
  group: string;
  overseasOnly?: boolean;
  hiddenInLibrary?: boolean;
  legacy?: boolean;
  note?: string;
  desc?: string;
  defaults: () => Record<string, any>;
  settings: SettingFieldDef[];
  summary?: (m: Record<string, any>) => string;
}


/** 深层取值（'title.titleText' 形式） */
export function getPath(obj: Record<string, any>, path: string): any {
  return path.split('.').reduce((acc, key) => (acc == null ? undefined : acc[key]), obj);
}

/** 设置项提示文案：hintFor 优先（随模块数据变化），否则回退静态 hint。 */
export function fieldHint(field: SettingFieldDef, moduleData: Record<string, any>): string {
  if (field.hintFor) return field.hintFor(moduleData);
  return field.hint || '';
}

/** 深层写入（不可变） */
export function setPath(obj: Record<string, any>, path: string, value: unknown): Record<string, any> {
  const keys = path.split('.');
  const clone: Record<string, any> = { ...obj };
  let cur = clone;
  for (let i = 0; i < keys.length - 1; i++) {
    const k = keys[i];
    cur[k] = { ...(cur[k] || {}) };
    cur = cur[k];
  }
  cur[keys[keys.length - 1]] = value;
  return clone;
}

export const containerFields = (opts: { showInnerPadding?: boolean } = {}): SettingFieldDef[] => [
  { kind: 'color', key: 'frame.backgroundColor', label: '背景颜色', resetValue: '#ffffff' },
  { kind: 'slider', key: 'frame.boxRadius', label: '容器圆角', max: 30 },
  { kind: 'slider', key: 'frame.boxPadding', label: '左右边距', max: 30 },
  { kind: 'slider', key: 'frame.boxPaddingTop', label: '上边距', max: 30 },
  { kind: 'slider', key: 'frame.boxPaddingBottom', label: '下边距', max: 30 },
  ...(opts.showInnerPadding
    ? [{ kind: 'slider' as const, key: 'frame.leftAndRightPadding', label: '左右内边距', max: 30 }]
    : []),
];

/** 购买按钮统一枚举选项（与 pc-semi 前台 lib/buy-style 的 BUY_BUTTON_OPTIONS 同表）。 */
export const BUY_BTN_STYLE_OPTIONS: Array<{ label: string; value: number }> = [
  { label: '跟随全局', value: -1 },
  { label: '不显示', value: 0 },
  { label: '购物车（描边）', value: 1 },
  { label: '购物车（实心）', value: 2 },
  { label: '加号（描边）', value: 3 },
  { label: '加号（实心）', value: 4 },
  { label: '“购买”描边按钮', value: 5 },
  { label: '“购买”圆角实心', value: 6 },
  { label: '“马上抢”描边按钮', value: 7 },
  { label: '“马上抢”圆角实心', value: 8 },
  { label: '圆圈购物车', value: 9 },
  { label: '自定义图片', value: 10 },
];

/** 购买按钮统一设置字段（商品类模块共用）：样式下拉 + 样式=自定义图片时出现的图标选图。
 *  key 缺省 'buyBtnStyle'（搜索结果模块传 'buyStyle'），图标字段缺省 'buyBtnIcon'。 */
export const buyBtnFields = (opts: { key?: string; iconKey?: string; section?: string } = {}): SettingFieldDef[] => {
  const key = opts.key ?? 'buyBtnStyle';
  const iconKey = opts.iconKey ?? 'buyBtnIcon';
  const section = opts.section ?? '购买按钮';
  return [
    {
      // 2026-11-21 改可视化点选（共享 BuyBtnStylePicker，用户口径：直接显示图标，
      // 不再用文字下拉）；kind/options/取值语义不变，仅控件形态变化。
      kind: 'buyBtnStyle',
      key,
      label: '购买按钮',
      section,
      options: BUY_BTN_STYLE_OPTIONS,
      buyBtnIconKey: iconKey,
      hint: '全站统一购买样式；「跟随全局」取后台「购买按钮样式设置」的全局值',
    },
    {
      kind: 'pic',
      key: iconKey,
      label: '自定义图标',
      section,
      showIf: (m) => Number(getPath(m, key)) === 10,
      hint: '样式为「自定义图片」时生效，可从素材库选择或上传（支持 SVG）；未设置时用全局自定义图标',
    },
  ];
};

/**
 * 标题设置字段集（对齐原版 _shared/ModuleTitleSettings.vue 的字段顺序/条件显隐/重置值/文案）：
 * 显示标题 → 标题样式 → 标题对齐 → 标题内容 → 标题颜色 → 描述内容 → 描述颜色 → 背景颜色
 * → 标题圆角 → [标题上/下边距] → 显示右侧内容/显示更多链接 → [右侧内容二选]
 * → 链接模式：[链接文字] → 链接颜色 → 链接地址 ｜ 倒计时模式：前缀文字 → 文字颜色
 *
 * `rootKey`：数据对象里标题对象的挂载路径。
 * - `'title'`（缺省）：字段挂在整模块对象上，key 形如 `title.showTitle`，showIf 走 `getPath(m,'title.x')`；
 * - `''`：字段挂在**标题对象本身**上（`kind:'title'` 渲染时会剥掉 `title.` 前缀并把 data 换成 titleData），
 *   showIf 直接读 `m.x`。
 *
 * 历史 bug：`kind:'title'` 分支把 data 换成了标题对象，但 showIf 仍按 `title.x` 取值，
 * 于是 `showTitleOn` 恒为 false，除「显示标题」外所有子项全部被隐藏（表现为标题设置缺项）。
 */
export const titleFields = (opts: {
  moreType?: boolean;
  verticalPadding?: boolean;
  rootKey?: string;
} = {}): SettingFieldDef[] => {
  const base = opts.rootKey === undefined ? 'title' : opts.rootKey;
  const at = (sub: string) => (base ? `${base}.${sub}` : sub);
  const moreColorDefault = opts.moreType ? '#1a1a1a' : '#000000';
  const showTitleOn = (m: Record<string, any>) => Number(getPath(m, at('showTitle'))) === 1;
  const showMoreOn = (m: Record<string, any>) => showTitleOn(m) && Number(getPath(m, at('showMore'))) === 1;
  const countdownMode = (m: Record<string, any>) =>
    showMoreOn(m) && !!opts.moreType && Number(getPath(m, at('moreType'))) === 2;
  const linkMode = (m: Record<string, any>) =>
    showMoreOn(m) && (!opts.moreType || Number(getPath(m, at('moreType'))) !== 2);
  const descVisible = (m: Record<string, any>) => showTitleOn(m) && Boolean(getPath(m, at('descText')));
  return [
    { kind: 'switch', key: at('showTitle'), label: '显示标题' },
    { kind: 'radio', key: at('titleStyle'), label: '标题样式', options: [{ label: '样式一', value: 1 }, { label: '样式二', value: 2 }, { label: '样式三', value: 3 }], showIf: showTitleOn },
    { kind: 'radio', key: at('titleAlign'), label: '标题对齐', options: [{ label: '左对齐', value: 1 }, { label: '居中', value: 2 }, { label: '右对齐', value: 3 }], showIf: showTitleOn },
    { kind: 'slider', key: at('titleFontSize'), label: '标题字号', min: 12, max: 48, defaultValue: 18, showIf: showTitleOn },
    { kind: 'input', key: at('titleText'), label: '标题内容', showIf: showTitleOn },
    { kind: 'color', key: at('titleColor'), label: '标题颜色', resetValue: '#2a3145', showIf: showTitleOn },
    { kind: 'input', key: at('descText'), label: '描述内容', showIf: showTitleOn },
    { kind: 'color', key: at('descColor'), label: '描述颜色', resetValue: '#aaaaaa', showIf: showTitleOn },
    { kind: 'slider', key: at('descFontSize'), label: '描述字号', min: 10, max: 32, defaultValue: 13, showIf: descVisible },
    { kind: 'radio', key: at('descLayout'), label: '描述排版', options: [{ label: '跟随标题后', value: 1 }, { label: '另起一行', value: 2 }], showIf: descVisible },
    { kind: 'color', key: at('titleBackground'), label: '背景颜色', showIf: showTitleOn },
    { kind: 'slider', key: at('titleRadius'), label: '标题圆角', max: 30, showIf: showTitleOn },
    // 标题左右边距：0 = 紧贴模块最左/最右；默认 10 与旧样式一致；上限 60（比通用 30 更宽，便于大留白）
    { kind: 'slider', key: at('titlePaddingLeft'), label: '标题左边距', max: 60, defaultValue: 10, showIf: showTitleOn },
    { kind: 'slider', key: at('titlePaddingRight'), label: '标题右边距', max: 60, defaultValue: 10, showIf: showTitleOn },
    ...(opts.verticalPadding
      ? [
          { kind: 'slider' as const, key: at('titlePaddingTop'), label: '标题上边距', max: 60 },
          { kind: 'slider' as const, key: at('titlePaddingBottom'), label: '标题下边距', max: 60 },
        ]
      : []),
    { kind: 'switch', key: at('showMore'), label: opts.moreType ? tl('显示右侧内容') : '显示更多链接', showIf: showTitleOn },
    ...(opts.moreType
      ? [
          { kind: 'radio' as const, key: at('moreType'), label: '右侧内容', options: [{ label: '显示更多', value: 1 }, { label: '显示倒计时', value: 2 }], showIf: showMoreOn },
          { kind: 'input' as const, key: at('countdownLabel'), label: '前缀文字', showIf: countdownMode },
        ]
      : []),
    { kind: 'input', key: at('moreLink.title'), label: '链接文字', showIf: linkMode },
    { kind: 'color', key: at('moreColor'), label: opts.moreType ? tl('文字颜色') : '链接颜色', resetValue: moreColorDefault, showIf: countdownMode },
    { kind: 'color', key: at('moreColor'), label: '链接颜色', resetValue: moreColorDefault, showIf: (m) => linkMode(m) && (!opts.moreType || Number(getPath(m, at('moreType'))) !== 2) },
    { kind: 'link', key: at('moreLink'), label: '链接地址', showIf: linkMode },
  ];
};

/* ===================== 新版模块集（modules/pcHome 20 个，kebab-case） ===================== */


/** 模块注册表（合并线接线时消费；重复 type 以先注册为准并告警） */
const pcModuleRegistry = new Map<string, ModuleDef>();
export function registerPcModule(def: ModuleDef): void {
  if (pcModuleRegistry.has(def.type)) {
    // eslint-disable-next-line no-console
    console.warn(`[pc-editor] duplicate module type: ${def.type}`);
    return;
  }
  pcModuleRegistry.set(def.type, def);
}
export function getPcModuleDef(type: string): ModuleDef | undefined {
  return pcModuleRegistry.get(type);
}
export function allPcModuleDefs(): ModuleDef[] {
  return [...pcModuleRegistry.values()];
}
