/** pcCoupon 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 LEGACY_MODULE_DEFS（PcDecorateEditor.tsx 第 1163 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCoupon', label: '优惠券模块', group: '营销活动', note: '内容由营销-优惠券"首页显示"决定', defaults: () => ({}), settings: [], summary: () => '优惠券' };

registerPcModule(def);

export default def;
