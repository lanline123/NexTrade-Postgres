/**
 * PC 装修模块 · pcGroupBanner（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1154），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcGroupBanner', label: '分组轮播Banner', group: '图片与轮播', defaults: () => ({ bannerStyle: 1, showCat: 1, logo: { picUrl: '', picThumb: '' }, picList: [], picList2: [], picList3: [], number: undefined as unknown as number }), settings: [{ kind: 'radio', key: 'bannerStyle', label: 'Banner风格', options: [{ label: '默认风格', value: 1 }, { label: '简约风格', value: 2 }] }, { kind: 'switch', key: 'showCat', label: '显示分类' }, { kind: 'pic', key: 'logo', label: 'LOGO' }, { kind: 'picList', key: 'picList', label: 'Banner图' }, { kind: 'number', key: 'number', label: '一组显示数量', min: 1, max: 3 }], summary: (m) => `每组 ${m.number || '-'}` };

registerPcModule(def);
export default def;
