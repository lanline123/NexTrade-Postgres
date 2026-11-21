/**
 * PC 装修模块 · pcBanner（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1152），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcBanner', label: '轮播Banner', group: '图片与轮播', defaults: () => ({ bannerStyle: 1, showCat: 1, logo: { picUrl: '', picThumb: '' }, picList: [], picList2: [], picList3: [] }), settings: [{ kind: 'radio', key: 'bannerStyle', label: '风格', options: [{ label: '默认风格', value: 1 }, { label: '简约风格', value: 2 }] }, { kind: 'switch', key: 'showCat', label: '显示分类' }, { kind: 'pic', key: 'logo', label: 'LOGO' }, { kind: 'picList', key: 'picList', label: '轮播大图' }, { kind: 'picList', key: 'picList2', label: '轮播图2' }, { kind: 'picList', key: 'picList3', label: '轮播图3' }], summary: (m) => `${((m.picList || []) as PicItem[]).length} 图` };

registerPcModule(def);
export default def;
