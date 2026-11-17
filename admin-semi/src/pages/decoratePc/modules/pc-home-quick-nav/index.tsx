/** PC 装修模块 pc-home-quick-nav（自 PcDecorateEditor.tsx NEW_MODULE_DEFS 逐字拆出，Step 3 模块化） */
import { containerFields, QUICK_NAV_ITEMS, registerPcModule, type ModuleDef, type PicItem } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-home-quick-nav', label: '快捷导航', group: '导航分类',
  desc: '横向图文导航条，支持图标+文字+链接配置',
  defaults: () => ({
    frame: { boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0, backgroundColor: '' },
    style: { fontSize: 14, textColor: '#333333', hoverColor: '#e1251b', containerAlign: 2, itemGap: 16, singleLine: 1, fontWeight: 700, lineHeight: 36, iconSize: 18, iconGap: 6 },
    items: QUICK_NAV_ITEMS,
  }),
  settings: [
    { kind: 'picList', key: 'items', label: '导航项', hint: '图标+文字+链接，可拖拽排序', picAddText: '添加导航项' },
    { kind: 'slider', key: 'style.fontSize', label: '字号', min: 12, max: 24 },
    { kind: 'radio', key: 'style.fontWeight', label: '文字粗细', options: [{ label: '常规体', value: 400 }, { label: '加粗体', value: 700 }] },
    { kind: 'slider', key: 'style.lineHeight', label: '行高', min: 12, max: 40 },
    { kind: 'color', key: 'style.textColor', label: '文字颜色', resetValue: '#333333' },
    { kind: 'color', key: 'style.hoverColor', label: '悬停颜色', resetValue: '#e1251b' },
    { kind: 'slider', key: 'style.iconSize', label: '图标尺寸', min: 12, max: 32 },
    { kind: 'slider', key: 'style.iconGap', label: '图文间距', min: 0, max: 16 },
    { kind: 'radio', key: 'style.containerAlign', label: '整体排布', options: [{ label: '居左', value: 1 }, { label: '居中', value: 2 }, { label: '居右', value: 3 }] },
    { kind: 'slider', key: 'style.itemGap', label: '项目间距', min: 8, max: 60 },
    { kind: 'radio', key: 'style.singleLine', label: '单行模式', options: [{ label: '单行不换行', value: 1 }, { label: '自动换行', value: 0 }] },
    ...containerFields(),
  ],
  summary: (m) => `${((m.items || []) as PicItem[]).length} 项`,
};

registerPcModule(def);

export default def;
