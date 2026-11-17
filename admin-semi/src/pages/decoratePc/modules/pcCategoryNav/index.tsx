/** PC 装修模块 pcCategoryNav（自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS 逐字拆出，Step 3 模块化） */
import { registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCategoryNav', label: '分类导航', group: '旧版', legacy: true, defaults: () => ({ rowNum: 5, categoryList: [] as number[] }), settings: [{ kind: 'radio', key: 'rowNum', label: '每屏显示数量', options: [2, 3, 4, 5, 6].map((n) => ({ label: `${n}个`, value: n })) }, { kind: 'categoryMulti', key: 'categoryList', label: '已选分类' }], summary: (m) => `${((m.categoryList || []) as number[]).length} 分类` };

registerPcModule(def);

export default def;
