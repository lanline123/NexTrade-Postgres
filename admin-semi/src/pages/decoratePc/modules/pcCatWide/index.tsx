/** PC 装修模块 pcCatWide（自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS 逐字拆出，Step 3 模块化） */
import { registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCatWide', label: '分类模块-宽', group: '商品展示', defaults: () => ({ catIds: [] as number[] }), settings: [{ kind: 'categoryMulti', key: 'catIds', label: '已选分类' }], summary: (m) => `${((m.catIds || []) as number[]).length} 分类` };

registerPcModule(def);

export default def;
