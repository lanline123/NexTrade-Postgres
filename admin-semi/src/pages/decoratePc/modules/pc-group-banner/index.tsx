/**
 * PC 装修模块 · pc-group-banner（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 590–609），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { containerFields, demoPic, DEMO_PIC_1, DEMO_PIC_2, registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-group-banner', label: '分组轮播Banner', group: '轮播横幅', overseasOnly: true,
  defaults: () => ({
    frame: { backgroundColor: '#ffffff', boxPadding: 10, boxPaddingTop: 0, boxPaddingBottom: 0, boxRadius: 8, slideHeight: 195 },
    bannerStyle: 1, showCat: 1, logo: { picUrl: '', picThumb: '' },
    picList: [demoPic('1', DEMO_PIC_1, '演示图片1', 'Banner1'), demoPic('2', DEMO_PIC_2, '演示图片2', 'Banner2')],
    picList2: [], picList3: [], number: 3,
    // 图片边角：1=直角(0px)、2=圆角(8px)。老数据无此字段时前台/预览均回退「容器圆角」，
    // 与原版 PcGroupBanner.vue 的 `__link{border-radius:var(--dec-box-radius)}` 一致，不产生跳变。
    picRadioStyle: 2,
  }),
  settings: [
    { kind: 'picList', key: 'picList', label: 'Banner图', hint: '可拖拽排序，建议 1180*940（高清）或 750*479' },
    { kind: 'number', key: 'number', label: '一组显示的数量', min: 1, max: 3, hint: '最多只显示3个' },
    // 独立于「容器圆角」（容器圆角管外框/整体裁切，本项只管单张图片圆角）
    { kind: 'radio', key: 'picRadioStyle', label: '图片边角', options: [{ label: '直角', value: 1 }, { label: '圆角', value: 2 }] },
    ...containerFields(),
  ],
  summary: (m) => `每组 ${m.number || '-'} 个 · ${((m.picList || []) as PicItem[]).length} 图`,
};

registerPcModule(def);
export default def;
