/**
 * PC 装修模块 · pcImageAd（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1155），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcImageAd', label: '通宽广告图', group: '图片与轮播', defaults: () => ({ picList: [], boxPaddingTop: 10, boxPaddingBottom: 10 }), settings: [{ kind: 'picList', key: 'picList', label: '广告图' }, { kind: 'slider', key: 'boxPaddingTop', label: '上边距', max: 30 }, { kind: 'slider', key: 'boxPaddingBottom', label: '下边距', max: 30 }], summary: (m) => `${((m.picList || []) as PicItem[]).length} 图` };

registerPcModule(def);
export default def;
