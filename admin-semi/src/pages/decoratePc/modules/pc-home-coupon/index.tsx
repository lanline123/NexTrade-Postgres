/** pc-home-coupon 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 NEW_MODULE_DEFS（PcDecorateEditor.tsx 第 928–950 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule, standardTitle } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-home-coupon', label: '优惠券', group: '营销活动',
  note: '优惠券内容由营销-优惠券"首页显示"决定',
  defaults: () => ({
    showTitle: 0, desc: '天天优惠领不停', colorStyle: 0,
    titleFontSize: 18, descFontSize: 12, titleAlign: 1, descLayout: 1,
    title: standardTitle({ titleText: '每日领券', descText: '天天优惠领不停' }),
    accentColor: '#16a34a', itemBackgroundColor: '#ffffff', backgroundColor: '#ffffff',
    itemPadding: 10, boxRadius: 0, boxPadding: 10, boxPaddingTop: 5, boxPaddingBottom: 5,
  }),
  settings: [
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    { kind: 'color', key: 'accentColor', label: '券主色（面额/按钮）', resetValue: '#16a34a' },
    { kind: 'color', key: 'itemBackgroundColor', label: '券卡背景色', resetValue: '#ffffff' },
    { kind: 'color', key: 'frame.backgroundColor', label: '模块背景色', resetValue: '#ffffff' },
    { kind: 'slider', key: 'itemPadding', label: '组件边距', max: 30, extend: true },
    { kind: 'slider', key: 'boxRadius', label: '圆角', max: 30, extend: true },
    { kind: 'slider', key: 'boxPadding', label: '左右边距', max: 30, extend: true },
    { kind: 'slider', key: 'boxPaddingTop', label: '上边距', max: 30, extend: true },
    { kind: 'slider', key: 'boxPaddingBottom', label: '下边距', max: 30, extend: true },
  ],
  summary: () => '展示"首页显示"的优惠券',
};

registerPcModule(def);

export default def;
