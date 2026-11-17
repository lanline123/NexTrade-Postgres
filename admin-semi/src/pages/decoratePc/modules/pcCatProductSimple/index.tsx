/**
 * PC 装修模块 · pcCatProductSimple（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1160），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCatProductSimple', label: '分类商品-简约', group: '旧版', legacy: true, defaults: () => ({ baseStyle: 1, title: '', products: { productSelectType: 1, productCategoryId: 1, productNumber: 8, productIds: [], productTag: '' }, moreLink: '', picList: [] }), settings: [{ kind: 'radio', key: 'baseStyle', label: '风格选择', options: [{ label: '风格一', value: 1 }, { label: '风格二', value: 2 }] }, { kind: 'input', key: 'title', label: '标题名称' }, { kind: 'link', key: 'moreLink', label: '更多链接' }, { kind: 'picList', key: 'picList', label: '添加左侧大图' }, { kind: 'products', key: 'products', label: '商品设置' }], summary: (m) => String(m.title || '') };

registerPcModule(def);
export default def;
