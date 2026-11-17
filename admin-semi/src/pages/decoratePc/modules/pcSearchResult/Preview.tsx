import { useI18n } from '../../../../i18n';
import { DEMO_PIC_1, DEMO_PIC_2 } from '../../contract/pcModuleContract';
import { PcBuyButtonVisual, resolvePcBuyBtn, usePcGlobalBuyStyle, SEARCH_LEGACY_MAP } from '../../previews/shared/PcBuyButton';

const PC_SEARCH_RESULT_DEFAULTS = {
  backgroundColor: '#ffffff',
  themeColor: '#ff4d4f',
  filterStyle: 2,
  filterTemplate: 2,
  goodsStyle: 1,
  goodsRadiusStyle: 1,
  // 购买按钮改全站统一枚举：新默认 5（"购买"描边）≡ 旧默认 buyStyle=2 的观感；buyStyleV=2 标记新格式
  buyStyle: 5,
  buyStyleV: 2,
  buyBtnIcon: { picUrl: '', picThumb: '' } as { picUrl: string; picThumb: string } | null,
  showFilterPanel: true as boolean | number,
  showPriceRange: true as boolean | number,
  showSales: false as boolean | number,
  showShopTag: false as boolean | number,
  showCartButton: true as boolean | number,
  componentMarginTop: 24,
  componentMarginBottom: 24,
};

/** 搜索结果模块预览（pageType=6）：按模块设置渲染筛选区+排序条+商品网格示意，
 *  开关兼容 true/1 两种存量写法（与前台 normalizeBoolLike 同口径）。 */
export default function PcSearchResultPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const d = { ...PC_SEARCH_RESULT_DEFAULTS, ...(data || {}) };
  const on = (v: unknown) => v === true || Number(v) === 1;
  const theme = d.themeColor || '#ff4d4f';
  /* 购买按钮（全站统一共享组件）：buyStyleV>=2 时 buyStyle 已是统一枚举；
     存量旧值（1/2/3）先映射再解析，与前台 ProductGrid 同口径。 */
  const globalBuy = usePcGlobalBuyStyle();
  const searchBuy = resolvePcBuyBtn(Number(d.buyStyleV) >= 2 ? d.buyStyle : SEARCH_LEGACY_MAP[Number(d.buyStyle)] ?? 2, {
    moduleIcon: d.buyBtnIcon,
    global: globalBuy,
  });
  const cols = Number(d.filterTemplate) === 2 ? 6 : 5;
  const radius = Number(d.goodsRadiusStyle) === 2 ? 8 : 0;
  const cardBorder =
    Number(d.goodsStyle) === 2
      ? '1px solid #eef0f3'
      : Number(d.goodsStyle) === 3
        ? '1px solid #d9dce1'
        : '1px solid transparent';
  const cardShadow = Number(d.goodsStyle) === 2 || Number(d.goodsStyle) === 4 ? '0 2px 10px rgba(16,24,40,0.06)' : 'none';
  const demo = [
    { name: '动力电池贴胶模组电芯拆分设备', price: '¥6.5万', sales: '已售 32', shop: '万物新生' },
    { name: '新能源汽车动力电池包回收处理线', price: '¥12.8万', sales: '已售 18', shop: '万物新生' },
    { name: '电池模组激光清洗设备', price: '¥3.6万', sales: '已售 45', shop: '绿源装备' },
    { name: '退役电池梯次利用检测台', price: '¥9800', sales: '已售 61', shop: '绿源装备' },
    { name: '电芯分容柜 512 通道', price: '¥2.4万', sales: '已售 27', shop: '华芯科技' },
    { name: '电池包气密性检测仪', price: '¥1.9万', sales: '已售 39', shop: '华芯科技' },
    { name: '模组堆叠自动化工作站', price: '¥8.8万', sales: '已售 12', shop: '万物新生' },
    { name: '电池回收破碎分选一体机', price: '¥15万', sales: '已售 9', shop: '绿源装备' },
    { name: '电芯外观缺陷视觉检测机', price: '¥4.2万', sales: '已售 23', shop: '华芯科技' },
    { name: '动力电池放电处理设备', price: '¥7600', sales: '已售 54', shop: '万物新生' },
    { name: '电池极片涂布回收装置', price: '¥5.5万', sales: '已售 15', shop: '绿源装备' },
    { name: '储能电池簇测试系统', price: '¥9.6万', sales: '已售 11', shop: '华芯科技' },
  ];
  const filterGroups = (
    <>
      <div style={{ marginBottom: 10 }}>
        <div style={{ color: '#999', fontSize: 12, marginBottom: 6 }}>{t('分类')}</div>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {[t('全部'), t('拆分设备'), t('检测设备'), t('回收处理')].map((x, i) => (
            <span key={x} style={{ padding: '3px 10px', fontSize: 12, borderRadius: 3, background: i === 0 ? theme : '#f5f6f8', color: i === 0 ? '#fff' : '#333' }}>{x}</span>
          ))}
        </div>
      </div>
      <div>
        <div style={{ color: '#999', fontSize: 12, marginBottom: 6 }}>{t('品牌')}</div>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {[t('全部'), t('万物新生'), t('绿源'), t('华芯')].map((x, i) => (
            <span key={x} style={{ padding: '3px 10px', fontSize: 12, borderRadius: 3, background: i === 0 ? theme : '#f5f6f8', color: i === 0 ? '#fff' : '#333' }}>{x}</span>
          ))}
        </div>
      </div>
      {on(d.showPriceRange) ? (
        <div style={{ marginTop: 10, display: 'flex', alignItems: 'center', gap: 6, fontSize: 12, color: '#666' }}>
          <span>{t('价格')}</span>
          <span style={{ border: '1px solid #e3e5e8', borderRadius: 3, padding: '3px 8px', color: '#bbb' }}>¥ {t('最低')}</span>
          <span>-</span>
          <span style={{ border: '1px solid #e3e5e8', borderRadius: 3, padding: '3px 8px', color: '#bbb' }}>¥ {t('最高')}</span>
          <span style={{ background: theme, color: '#fff', borderRadius: 3, padding: '4px 10px' }}>{t('确定')}</span>
        </div>
      ) : null}
    </>
  );
  const sortBar = (
    <div style={{ display: 'flex', alignItems: 'center', gap: 18, borderBottom: '1px solid #eef0f3', padding: '8px 2px', fontSize: 13 }}>
      {[t('综合'), t('销量'), t('价格'), t('新品')].map((x, i) => (
        <span key={x} style={{ color: i === 0 ? theme : '#333', fontWeight: i === 0 ? 600 : 400 }}>{x}</span>
      ))}
      <span style={{ marginLeft: 'auto', color: '#999', fontSize: 12 }}>{t('共 {n} 件商品', { n: 132 })}</span>
    </div>
  );
  const grid = (
    <div style={{ display: 'grid', gridTemplateColumns: `repeat(${cols}, 1fr)`, gap: 12, marginTop: 12 }}>
      {demo.map((p, i) => (
        <div key={i} style={{ border: cardBorder, boxShadow: cardShadow, borderRadius: radius, overflow: 'hidden', background: '#fff' }}>
          <img src={i % 2 ? DEMO_PIC_2 : DEMO_PIC_1} alt="" style={{ width: '100%', aspectRatio: '1', objectFit: 'cover', display: 'block', borderRadius: radius }} />
          <div style={{ padding: 8 }}>
            <div style={{ fontSize: 12, lineHeight: 1.5, height: 36, overflow: 'hidden', color: '#333' }}>{p.name}</div>
            <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginTop: 4 }}>
              <span style={{ color: theme, fontWeight: 600, fontSize: 14 }}>{p.price}</span>
              {on(d.showSales) ? <span style={{ color: '#999', fontSize: 11 }}>{p.sales}</span> : null}
            </div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 6 }}>
              {on(d.showShopTag) ? <span style={{ fontSize: 11, color: '#888', border: '1px solid #e3e5e8', borderRadius: 2, padding: '1px 5px' }}>{p.shop}</span> : <span />}
              {on(d.showCartButton) && searchBuy.show ? (
                <span style={{ color: theme, display: 'inline-flex', alignItems: 'center' }}>
                  <PcBuyButtonVisual style={searchBuy.style} customIcon={searchBuy.customIcon} />
                </span>
              ) : null}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
  return (
    <div style={{ background: d.backgroundColor || '#fff', marginTop: Number(d.componentMarginTop) || 0, marginBottom: Number(d.componentMarginBottom) || 0, padding: '14px 16px 18px' }}>
      {Number(d.filterStyle) === 1 ? (
        <div style={{ display: 'flex', gap: 16 }}>
          {on(d.showFilterPanel) ? (
            <div style={{ width: 200, flexShrink: 0, borderRight: '1px solid #eef0f3', paddingRight: 14 }}>{filterGroups}</div>
          ) : null}
          <div style={{ flex: 1, minWidth: 0 }}>
            {sortBar}
            {grid}
          </div>
        </div>
      ) : (
        <>
          {on(d.showFilterPanel) ? <div style={{ borderBottom: '1px solid #eef0f3', paddingBottom: 10 }}>{filterGroups}</div> : null}
          {sortBar}
          {grid}
        </>
      )}
    </div>
  );
}
