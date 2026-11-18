/**
 * PC 装修模块 · pc-image-hotarea（Step 3 模块化：def 逐字拆自 PcDecorateEditor.tsx NEW_MODULE_DEFS（源行 704–719），合并线统一接线）
 */

import type { ModuleDef, PicItem } from '../../contract/pcModuleContract';
import { containerFields, registerPcModule, standardFrame, standardTitle } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-image-hotarea', label: '热图绘制', group: '图片与轮播',
  defaults: () => ({
    picList: [] as PicItem[],
    imgPadding: 0,
    frame: standardFrame(),
    title: standardTitle(),
  }),
  settings: [
    { kind: 'picList', key: 'picList', label: '热图', hint: '添加图片后，在下方「热区绘制」中直接在图上拖拽框选热区' },
    { kind: 'hotarea', key: 'hotarea', label: '热区绘制' },
    { kind: 'title', key: 'title', label: '标题设置', titleHideMoreType: true },
    ...containerFields({ showInnerPadding: true }),
  ],
  summary: (m) => `${((m.picList || []) as PicItem[]).length} 图`,
};

registerPcModule(def);
export default def;
