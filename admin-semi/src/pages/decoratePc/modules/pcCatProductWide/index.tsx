/**
 * PC 装修模块 · pcCatProductWide（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS（源行 1159），合并线统一接线）
 */

import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCatProductWide', label: '分类商品-宽', group: '旧版', legacy: true, defaults: () => ({ categoryId: 0, childCategoryIds: [] as number[], title: '', shortTitle: '', enTitle: '', color: '', color2: '', logo: { picUrl: '', picThumb: '' }, picList: [], picList2: [], productIds: [], linkList: [] }), settings: [{ kind: 'categoryMulti', key: 'categoryId', label: '选择分类' }, { kind: 'categoryMulti', key: 'childCategoryIds', label: '选择子分类显示' }, { kind: 'input', key: 'title', label: '标题名称' }, { kind: 'input', key: 'shortTitle', label: '标题短名称' }, { kind: 'input', key: 'enTitle', label: '标题英文简称' }, { kind: 'color', key: 'color', label: '模块主颜色' }, { kind: 'color', key: 'color2', label: '模块副颜色(渐变)' }, { kind: 'picList', key: 'picList', label: '左侧轮播图', picDesc: true }, { kind: 'linkList', key: 'linkList', label: '左侧轮播下方链接' }, { kind: 'picList', key: 'picList2', label: '中间6广告图', picDesc: true }, { kind: 'productIds', key: 'productIds', label: '右侧热门商品' }], summary: (m) => String(m.title || '') };

registerPcModule(def);
export default def;
