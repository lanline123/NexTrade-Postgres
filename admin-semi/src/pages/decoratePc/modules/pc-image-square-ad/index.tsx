/**
 * PC 装修模块 · pc-image-square-ad（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 683–703），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { containerFields, demoPic, DEMO_PIC_1, DEMO_PIC_2, registerPcModule, standardFrame, standardTitle } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-image-square-ad', label: '图片魔方', group: '图片展示',
  defaults: () => ({
    picType: 1,
    picList: [demoPic('1', DEMO_PIC_1, '演示图片1', '魔方图1'), demoPic('2', DEMO_PIC_2, '演示图片2', '魔方图2')],
    imgPadding: 0,
    frame: standardFrame(),
    title: standardTitle(),
  }),
  settings: [
    { kind: 'squareTemplate', key: 'picType', label: '选择模板', section: '选择布局' },
    { kind: 'picList', key: 'picList', label: '添加图片', section: '添加图片',
      // 原版：格子数量由布局决定（固定 2/3/4 张），标题处展示"固定 N 张"
      hintFor: (m) => `固定 ${({ 1: 2, 2: 3, 3: 4, 4: 4, 5: 3, 6: 4, 7: 3 } as Record<number, number>)[Number(m.picType)] ?? 2} 张（先选模板确定格子数量）`,
      picMaxFor: (m) => ({ 1: 2, 2: 3, 3: 4, 4: 4, 5: 3, 6: 4, 7: 3 } as Record<number, number>)[Number(m.picType)] ?? 2 },
    { kind: 'slider', key: 'imgPadding', label: '图片边距', max: 30 },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    ...containerFields({ showInnerPadding: true }),
  ],
  summary: (m) => `布局${m.picType} · ${((m.picList || []) as PicItem[]).length} 图`,
};

registerPcModule(def);
export default def;
