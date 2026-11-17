/** PC 装修模块 pcCategoryNav3（自 PcDecorateEditor.tsx LEGACY_MODULE_DEFS 逐字拆出，Step 3 模块化） */
import { registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

const def: ModuleDef = { type: 'pcCategoryNav3', label: '分类导航3', group: '旧版', legacy: true, defaults: () => ({ bannerStyle: 1, showCat: 1, logo: { picUrl: '', picThumb: '' }, picList: [], picList2: [], picList3: [], imageBackgroundColor: undefined as unknown as string, titleText: undefined as unknown as string, showMore: undefined as unknown as number, showMoreText: undefined as unknown as string, moreLink: undefined as unknown as Record<string, any> }), settings: [{ kind: 'picList', key: 'picList', label: '图片卡片' }, { kind: 'color', key: 'imageBackgroundColor', label: '图片背景色' }, { kind: 'input', key: 'titleText', label: '标题内容' }, { kind: 'switch', key: 'showMore', label: '查看更多' }], summary: (m) => String(m.titleText || '') };

registerPcModule(def);

export default def;
