/** PC 装修模块 pc-category-nav3（自 PcDecorateEditor.tsx NEW_MODULE_DEFS 逐字拆出，Step 3 模块化） */
import { containerFields, getPath, registerPcModule, standardTitle, type ModuleDef, type PicItem } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-category-nav3', label: '分类导航3', group: '导航分类', overseasOnly: true,
  desc: '选择图片和链接，可编辑文字，方便轻松设置',
  defaults: () => ({
    frame: { boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0, boxRadius: 8, backgroundColor: 'rgba(0, 0, 0, 0.02)', boxHeight: 352 },
    titleText: '热门推荐', showMore: 1, showMoreText: '更多', moreLink: { link: '', name: '' },
    title: standardTitle({ showTitle: 1, titleText: '热门推荐', showMore: 1, moreLink: { link: '', title: '更多' } }),
    imageBackgroundColor: '', picList: [], backgroundColor: 'rgba(0, 0, 0, 0.02)',
  }),
  settings: [
    { kind: 'picList', key: 'picList', label: '图片卡片', hint: '可拖拽排序，建议 180*180 像素', picDesc: true, picAddText: '添加一组' },
    { kind: 'color', key: 'imageBackgroundColor', label: '图片背景色', resetValue: '', section: '参数设置' },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    { kind: 'color', key: 'backgroundColor', label: '模块背景颜色', resetValue: 'rgba(0, 0, 0, 0.02)' },
    // 扩展设置：frame.* 自动归入「扩展设置」Tab。前台/预览均读取 frame.boxHeight 与容器边距，
    // 此前编辑器无入口，线上 10023 的 boxHeight=300 只能在数据层修改。
    // 模块已有独立的「模块背景颜色」（前台 backgroundColor 优先于 frame.backgroundColor），容器背景色不再重复。
    { kind: 'slider', key: 'frame.boxHeight', label: '组件高度', min: 200, max: 600, defaultValue: 352 },
    ...containerFields().filter((f) => f.key !== 'frame.backgroundColor'),
  ],
  summary: (m) => `${getPath(m, 'title.titleText') || m.titleText || ''} · ${((m.picList || []) as PicItem[]).length} 卡`,
};

registerPcModule(def);

export default def;
