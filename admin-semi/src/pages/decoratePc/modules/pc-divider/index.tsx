import { containerFields, registerPcModule, tl, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-divider', label: '分割线', group: '基础排版',
  defaults: () => ({
    lineType: 2, lineStyle: 1,
    frame: { textColor: '', itemBackgroundColor: '', backgroundColor: '#ffffff', innerPadding: 0, itemHeight: 10, itemRadius: 0, boxRadius: 0, boxPadding: 10, boxPaddingTop: 5, boxPaddingBottom: 5 },
  }),
  settings: [
    { kind: 'radio', key: 'lineType', label: '分割类型', options: [{ label: '空白', value: 1 }, { label: '分割线', value: 2 }] },
    { kind: 'radio', key: 'lineStyle', label: '选择样式', options: [{ label: '实线', value: 1 }, { label: '虚线', value: 2 }, { label: '点状线', value: 3 }] },
    // 原版：组件高度在扩展设置顶部（无分组标题），容器设置标题之后才是一般容器项
    { kind: 'slider', key: 'frame.itemHeight', label: '组件高度', min: 0, max: 100 },
    ...containerFields().map((f, index) => (index === 0 ? { ...f, section: '容器设置' } : f)),
  ],
  summary: (m) => (Number(m.lineType) === 1 ? tl('空白') : '分割线'),
};

registerPcModule(def);
export default def;
