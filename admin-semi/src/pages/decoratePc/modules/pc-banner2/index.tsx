/**
 * PC 装修模块 · pc-banner2（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 573–589），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { containerFields, demoPic, DEMO_PIC_1, DEMO_PIC_2, registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-banner2', label: '轮播Banner2', group: '轮播横幅', overseasOnly: true,
  defaults: () => ({
    frame: { boxPadding: 10, boxPaddingTop: 0, boxPaddingBottom: 0, boxRadius: 8, backgroundColor: '#eeeeee' },
    bannerStyle: 1, showCat: 1, logo: { picUrl: '', picThumb: '' },
    picList: [demoPic('1', DEMO_PIC_1, '演示图片1', 'Banner1'), demoPic('2', DEMO_PIC_2, '演示图片2', 'Banner2')],
    picList2: [], picList3: [], number: 1, slideGap: 12,
  }),
  settings: [
    { kind: 'picList', key: 'picList', label: '轮播图', hint: '可拖拽排序，建议尺寸 1264*280' },
    // 组件高度：原版无此设置（高度跟随图片比例），按用户要求新增。
    // 0 = 跟随图片比例自适应（线上老数据无 frame.boxHeight，默认即 0）。
    { kind: 'slider', key: 'frame.boxHeight', label: '组件高度', min: 0, max: 600, defaultValue: 0, hint: '0 = 高度跟随图片比例自适应' },
    ...containerFields(),
  ],
  summary: (m) => `${((m.picList || []) as PicItem[]).length} 图`,
};

registerPcModule(def);
export default def;
