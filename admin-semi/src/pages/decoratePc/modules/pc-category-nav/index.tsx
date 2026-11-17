/** PC 装修模块 pc-category-nav（自 PcDecorateEditor.tsx NEW_MODULE_DEFS 逐字拆出，Step 3 模块化） */
import { containerFields, getPath, registerPcModule, standardTitle, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-category-nav', label: '分类导航', group: '导航分类',
  defaults: () => ({
    frame: { boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0, boxRadius: 8, backgroundColor: '#f7f7f7', boxHeight: 328 },
    titleName: '热门分类', showTitle: 1, displayNum: 6, backgroundColor: '#f7f7f7',
    title: standardTitle({ showTitle: 1, titleText: '热门分类' }),
  }),
  settings: [
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    { kind: 'slider', key: 'displayNum', label: '一屏显示数量', min: 1, max: 10 },
    { kind: 'color', key: 'backgroundColor', label: '背景颜色', resetValue: '#f7f7f7' },
    ...containerFields(),
    // 组件高度：线上老数据无 frame.boxHeight，缺省显示 328（与前台 CSS 固定值一致）。
    // pc-semi 的 CategoryNav 同步改为读取 frame.boxHeight，避免「后台可改、C 端不跟随」。
    { kind: 'slider', key: 'frame.boxHeight', label: '组件高度', min: 200, max: 600, defaultValue: 328 },
  ],
  summary: (m) => `${getPath(m, 'title.titleText') || m.titleName || ''} · 每屏 ${m.displayNum}`,
};

registerPcModule(def);

export default def;
