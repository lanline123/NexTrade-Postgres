import { containerFields, registerPcModule, standardFrame, standardTitle, tl, type ModuleDef, type PicItem } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-image-nav', label: '图文导航', group: '导航入口',
  defaults: () => ({
    navType: 1, navStyle: 1, rowNum: 4, colNum: 2, picList: [],
    backgroundColor: '#ffffff', imgPadding: 8, radioStyle: 1, picPageType: 1, swiperPageColor: '#333333',
    // 标题设置块：原版 PcImageNav.vue 无标题，画布预览早已接入共享 ModuleTitlePreview
    // （PcImageNavPreview:227）却无 data 与入口 → 此处补齐，默认不显示，零视觉变化。
    title: standardTitle(),
    frame: standardFrame(),
  }),
  settings: [
    { kind: 'radio', key: 'navType', label: '显示模式', options: [{ label: '固定显示', value: 1 }, { label: '滑动分页', value: 2 }] },
    { kind: 'radio', key: 'navStyle', label: '样式', options: [{ label: '图文', value: 1 }, { label: '仅图片', value: 2 }, { label: '仅文字', value: 3 }] },
    { kind: 'select', key: 'rowNum', label: '一行显示', options: [3, 4, 5, 6].map((n) => ({ label: `${n}个`, value: n })) },
    { kind: 'select', key: 'colNum', label: '显示行数', options: [1, 2, 3].map((n) => ({ label: `${n}行`, value: n })) },
    { kind: 'picList', key: 'picList', label: '导航项', hint: '可拖拽排序', picAddTextFor: (m) => Number(m.navStyle) === 3 ? tl('添加导航项') : '添加图片' },
    { kind: 'color', key: 'backgroundColor', label: '背景颜色', resetValue: '#ffffff' },
    { kind: 'slider', key: 'imgPadding', label: '元素边距', max: 30 },
    // 原版 isNavStyleImage = navStyle !== 3：仅文字模式下不显示图片边角
    { kind: 'radio', key: 'radioStyle', label: '图片边角', options: [{ label: '直角', value: 1 }, { label: '圆角', value: 2 }], showIf: (m) => Number(m.navStyle) !== 3 },
    { kind: 'radio', key: 'picPageType', label: '指示器样式', options: [{ label: '样式一', value: 1 }, { label: '样式二', value: 2 }], showIf: (m) => Number(m.navType) === 2 },
    { kind: 'color', key: 'swiperPageColor', label: '指示器颜色', resetValue: '#333333', showIf: (m) => Number(m.navType) === 2 },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    ...containerFields(),
  ],
  summary: (m) => `${m.rowNum}×${m.colNum} · ${((m.picList || []) as PicItem[]).length} 项`,
};

registerPcModule(def);
export default def;
