/** pc-home-groupon 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 NEW_MODULE_DEFS（PcDecorateEditor.tsx 第 973–1009 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule, standardTitle, buyBtnFields } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-home-groupon', label: '拼团', group: '营销活动',
  defaults: () => ({
    style: 4, groupType: 1, cardStyle: 1, showName: 1, showPrice: 1, textAlign: 1, goodsPadding: 0,
    backgroundColor: '#ffffff', boxRadius: 0, innerPadding: 0, boxPadding: 0, boxPaddingTop: 5, boxPaddingBottom: 5,
    title: standardTitle({ titleText: '拼团', showMore: 1, moreType: 2, countdownLabel: '距结束', moreColor: '#1a1a1a' }),
    // 购买按钮（全站统一）：旧默认 0 在存量语义里是「未配置」，新模块显式写 4（加号实心）+ buyBtnSet 标记
    buyBtnStyle: 4,
    buyBtnSet: 1,
    buyBtnIcon: { picUrl: '', picThumb: '' },
    grouponActivityItem: null as unknown as Record<string, any>,
  }),
  settings: [
    { kind: 'radio', key: 'style', label: '选择模板', options: [{ label: '一行四个', value: 2 }, { label: '一行五个', value: 3 }, { label: '一行六个', value: 4 }, { label: '横向滑动', value: 6 }] },
    {
      kind: 'radio', key: 'groupType', label: '拼团类型', options: [{ label: '普通拼团', value: 1 }, { label: '阶梯拼团', value: 2 }],
      onChangeExtra: (m) => ({ ...m, grouponActivityItem: null }), // 原版：切换类型清空已选活动
    },
    { kind: 'groupon', key: 'grouponActivityItem', label: '选择活动' },
    { kind: 'radio', key: 'cardStyle', label: '商品卡片样式', options: [{ label: '样式一', value: 1 }, { label: '样式二', value: 2 }] },
    { kind: 'radio', key: 'textAlign', label: '文本对齐', options: [{ label: '左对齐', value: 1 }, { label: '居中对齐', value: 2 }] },
    { kind: 'radio', key: 'showName', label: '显示商品名称', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    { kind: 'radio', key: 'showPrice', label: '显示商品价格', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    { kind: 'slider', key: 'goodsPadding', label: '商品边距', max: 50 },
    // 购买按钮（全站统一）：旧版该字段只存数据、面板未暴露，现补齐统一字段
    ...buyBtnFields(),
    // 原版顺序：模板/类型/活动/卡片/边距在前，标题设置位于内容设置末尾
    { kind: 'title', key: 'title', label: '标题设置' },
    { kind: 'color', key: 'backgroundColor', label: '背景颜色', resetValue: '#ffffff', extend: true },
    { kind: 'slider', key: 'boxRadius', label: '圆角', max: 30, extend: true },
    { kind: 'slider', key: 'innerPadding', label: '内边距', max: 30, extend: true },
    { kind: 'slider', key: 'boxPadding', label: '左右边距', max: 30, extend: true },
    { kind: 'slider', key: 'boxPaddingTop', label: '上边距', max: 30, extend: true },
    { kind: 'slider', key: 'boxPaddingBottom', label: '下边距', max: 30, extend: true },
  ],
  summary: (m) => (m.grouponActivityItem ? m.grouponActivityItem.name : '未选择活动'),
};

registerPcModule(def);

export default def;
