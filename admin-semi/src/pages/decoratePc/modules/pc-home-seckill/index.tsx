/** pc-home-seckill 模块定义 —— 逐字摘自 PcDecorateEditor.tsx 的 NEW_MODULE_DEFS（PcDecorateEditor.tsx 第 951–972 行），Step 3 模块化拆分（P4 营销组）。 */
import type { ModuleDef } from '../../contract/pcModuleContract';
import { registerPcModule, standardTitle, buyBtnFields, containerFields, getPath } from '../../contract/pcModuleContract';

const def: ModuleDef = {
  type: 'pc-home-seckill', label: '秒杀', group: '营销活动',
  defaults: () => ({
    frame: { boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0, boxRadius: 0 },
    // seckillTitle 是迁移前的平铺字段：仅为老数据/旧链适配器（contract.ts toTitle(m.seckillTitle)）保留，
    // 前台已不再读取它（normalizeLegacyTitle 会把老值搬进 title.titleText）；新配置一律走 title 契约。
    seckillTitle: '全场优惠限时9折起',
    title: standardTitle({ showTitle: 1, titleText: '全场优惠限时9折起', showMore: 1, moreType: 2, countdownLabel: '倒计时', moreColor: '#1a1a1a' }),
    goodsGap: 10,
    // 购买按钮（全站统一）：新模块显式写 4（加号实心，与旧版恒显示加号的观感一致）+ buyBtnSet 标记
    buyBtnStyle: 4,
    buyBtnSet: 1,
    buyBtnIcon: { picUrl: '', picThumb: '' },
  }),
  settings: [
    { kind: 'slider', key: 'goodsGap', label: '商品间距', max: 40 },
    ...buyBtnFields(),
    { kind: 'title', key: 'title', label: '标题设置', hint: '秒杀商品由营销-秒杀决定' },
    ...containerFields({ showInnerPadding: true }),
  ],
  summary: (m) => String(getPath(m, 'title.titleText') || m.seckillTitle || ''),
};

registerPcModule(def);

export default def;
