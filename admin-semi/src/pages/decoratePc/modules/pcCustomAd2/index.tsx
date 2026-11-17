/** pcCustomAd2 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 LEGACY_MODULE_DEFS（PcDecorateEditor.tsx 第 1157 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCustomAd2', label: '自定义模块二', group: '旧版', legacy: true, defaults: () => ({ linkList: [], picList: [], picList2: [], picList3: [] }), settings: [{ kind: 'picList', key: 'picList', label: '左侧大图' }, { kind: 'linkList', key: 'linkList', label: '左侧链接' }, { kind: 'picList', key: 'picList2', label: '中间广告图', picDesc: true }, { kind: 'picList', key: 'picList3', label: '右侧广告图', picDesc: true }], summary: () => '三列自定义' };

registerPcModule(def);

export default def;
