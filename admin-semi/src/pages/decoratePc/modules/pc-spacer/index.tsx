import { containerFields, registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = {
    type: 'pc-spacer', label: '空白辅助', group: '基础排版',
    defaults: () => ({
      frame: { boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0, boxRadius: 0 },
      backgroundColor: '#ffffff',
      blankHeight: 30,
    }),
    settings: [
      { kind: 'color', key: 'backgroundColor', label: '背景颜色', resetValue: '#ffffff' },
      { kind: 'slider', key: 'blankHeight', label: '高度', min: 0, max: 200 },
      // 原版 ModuleContainerSettings hide-background-color：自身已有背景色字段，容器设置里不再重复
      ...containerFields().filter((f) => f.key !== 'frame.backgroundColor'),
    ],
    summary: (m) => `高 ${m.blankHeight}px`,
};

registerPcModule(def);
export default def;
