/** pcSeckill 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 LEGACY_MODULE_DEFS（PcDecorateEditor.tsx 第 1162 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcSeckill', label: '秒杀模块', group: '旧版', legacy: true, defaults: () => ({ seckillTitle: '全场优惠限时9折起' }), settings: [{ kind: 'input', key: 'seckillTitle', label: '活动标题' }], summary: (m) => String(m.seckillTitle || '') };

registerPcModule(def);

export default def;
