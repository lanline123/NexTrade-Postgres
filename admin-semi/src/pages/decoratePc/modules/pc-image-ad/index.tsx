/**
 * PC 装修模块 · pc-image-ad（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 610–655），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { containerFields, demoPic, DEMO_PIC_1, DEMO_PIC_2, registerPcModule, standardFrame, standardTitle } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-image-ad', label: '图片广告', group: '图片与轮播',
  defaults: () => ({
    picType: 1,
    picList: [demoPic('1', DEMO_PIC_1, '演示图片1', '广告位1'), demoPic('2', DEMO_PIC_2, '演示图片2', '广告位2')],
    swiperPreView: 1, swiperPageColor: '#333333', imgPadding: 0, picPageType: 1, picRadioStyle: 1, isFluxWidth: 0,
    frame: standardFrame(),
    title: standardTitle(),
  }),
  settings: [
    { kind: 'radio', key: 'picType', label: '选择模板', iconRadio: true,
      // 原版 pc-image-ad/Settings.vue：el-radio-button 内为 ico-decorate 图标 + el-tooltip 文案
      optionIcons: { 1: '\ue603', 2: '\ue601', 5: '\ue61c', 6: '\ue61d', 7: '\ue61b' },
      options: [
        { label: '一行一个', value: 1 },
        { label: '轮播', value: 2 },
        { label: '一行两个', value: 5 },
        { label: '一行三个', value: 6 },
        { label: '一行四个', value: 7 },
      ] },
    { kind: 'picList', key: 'picList', label: '图片列表', section: '添加图片',
      // 原版按 picType 显示不同提示文案（仅一行一个 / 轮播两种布局有提示）
      hintFor: (m) => {
        const picType = Number(m.picType);
        if (picType === 2) return '提示：您可以通过拖拽进行图片排序，宽不小于1500px（选择轮播时可开启通宽排版）';
        if (picType === 1) return '提示：您可以通过拖拽进行图片排序，单图占宽为100%，建议宽1500px，高度自适应';
        return '';
      } },
    { kind: 'radio', key: 'picPageType', label: '指示器样式', iconRadio: true, showIf: (m) => Number(m.picType) === 2,
      // 原版：el-radio-button + ico-decorate icon-dec-indicator-1/2/3 + el-tooltip「样式一/二/三」
      optionIcons: { 1: '\ue604', 2: '\ue606', 3: '\ue605' },
      options: [{ label: '样式一', value: 1 }, { label: '样式二', value: 2 }, { label: '样式三', value: 3 }] },
    { kind: 'color', key: 'swiperPageColor', label: '指示器颜色', resetValue: '#333333', showIf: (m) => Number(m.picType) === 2 },
    { kind: 'select', key: 'swiperPreView', label: '轮播一行显示', options: [1, 2, 3, 4].map((n) => ({ label: `${n}个`, value: n })), showIf: (m) => Number(m.picType) === 2 },
    { kind: 'slider', key: 'imgPadding', label: '图片边距', max: 30 },
    { kind: 'radio', key: 'picRadioStyle', label: '图片边角', options: [{ label: '直角', value: 1 }, { label: '圆角', value: 2 }] },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    { kind: 'radio', key: 'isFluxWidth', label: '是否通宽排版', options: [{ label: '是', value: 1 }, { label: '否', value: 0 }], extend: true },
    // 原版 ModuleContainerSettings hide-horizontal-spacing：通宽排版时隐藏"左右边距"
    ...containerFields().map((f, index) => ({
      ...(f.key === 'frame.boxPadding' ? { ...f, showIf: (m: Record<string, any>) => Number(m.isFluxWidth) !== 1 } : f),
      ...(index === 0 ? { section: '容器设置' } : {}),
    })),
  ],
  summary: (m) => `模板${m.picType} · ${((m.picList || []) as PicItem[]).length} 图`,
};

registerPcModule(def);
export default def;
