/**
 * PC 装修模块 · pcRecommendProduct（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1161），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcRecommendProduct', label: '推荐商品-瀑布流', group: '旧版', legacy: true, defaults: () => ({ titleName: '', products: { productNumber: 50, productIds: [] } }), settings: [{ kind: 'input', key: 'titleName', label: '标题' }, { kind: 'products', key: 'products', label: '商品设置', numberMin: 20, numberStep: 10 }, { kind: 'switch', key: 'showTitle', label: '显示标题' }], summary: (m) => String(m.titleName || '') };

registerPcModule(def);
export default def;
