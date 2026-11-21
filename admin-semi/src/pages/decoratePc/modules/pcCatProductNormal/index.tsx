/**
 * PC 装修模块 · pcCatProductNormal（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1158），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCatProductNormal', label: '分类商品-窄', group: '商品展示', defaults: () => ({ categoryId: 0, title: '', shortTitle: '', enTitle: '', color: '', color2: '', logo: { picUrl: '', picThumb: '' }, picList: [], picList2: [] }), settings: [{ kind: 'category', key: 'categoryId', label: '选择分类' }, { kind: 'input', key: 'title', label: '标题名称' }, { kind: 'input', key: 'shortTitle', label: '标题短名称' }, { kind: 'input', key: 'enTitle', label: '标题英文简称' }, { kind: 'color', key: 'color', label: '模块主颜色' }, { kind: 'color', key: 'color2', label: '模块副颜色(渐变)' }, { kind: 'picList', key: 'picList', label: '左侧轮播图', picDesc: true }, { kind: 'picList', key: 'picList2', label: '右侧四小图', picDesc: true }], summary: (m) => String(m.title || '') };

registerPcModule(def);
export default def;
