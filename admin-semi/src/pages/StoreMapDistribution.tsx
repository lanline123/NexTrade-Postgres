import { useEffect, useMemo, useState } from 'react';
import { Card, Empty, Spin, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import StoreDistributionMap, { type StoreMapPoint } from '../components/StoreDistributionMap';

const { Text, Title } = Typography;

/** 门店分布图行（后端 StoreMapVO）。 */
interface StoreMapItem extends StoreMapPoint {
  shopType?: number | null;
  status?: number | null;
  regionIds?: string | null;
  address?: string | null;
}

const statusMap: Record<number, { text: string; color: 'green' | 'red' | 'orange' | 'grey' }> = {
  1: { text: '开业', color: 'green' },
  4: { text: '停业', color: 'red' },
  10: { text: '打烊', color: 'orange' },
};

function isLocated(store: StoreMapItem) {
  const lng = Number(store.longitude);
  const lat = Number(store.latitude);
  return Number.isFinite(lng) && Number.isFinite(lat) && Boolean(lng) && Boolean(lat);
}

function radiusText(store: StoreMapItem) {
  const radius = Number(store.deliveryRadius) || 0;
  if (radius <= 0) return '未设置配送范围';
  if (store.deliveryEnabled === 0) return `配送半径 ${radius} 米（同城配送未开启）`;
  return `配送半径 ${radius} 米`;
}

/**
 * 门店分布图（组织 → 门店管理，我方新增，论坛帖 587859 余子项）。
 *
 * 一次拉全量门店（平台视角；店铺账号后端只回本店），地图上全部打点并
 * 叠加各自同城配送范围圈；右侧名单与地图联动选中，未设置经纬度的门店
 * 单列名单提示去门店信息里补地址。单店范围的编辑仍在原配送设置页。
 */
export default function StoreMapDistribution() {
  const [loading, setLoading] = useState(true);
  const [stores, setStores] = useState<StoreMapItem[]>([]);
  const [selectedShopId, setSelectedShopId] = useState<number | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<StoreMapItem[]>('/merchant/shop/storeMap');
        setStores(Array.isArray(data) ? data : []);
      } catch {
        Toast.error({ content: '加载门店分布失败' });
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const located = useMemo(() => stores.filter(isLocated), [stores]);
  const unlocated = useMemo(() => stores.filter((s) => !isLocated(s)), [stores]);
  const withRadius = useMemo(
    () => located.filter((s) => (Number(s.deliveryRadius) || 0) > 0 && s.deliveryEnabled !== 0),
    [located],
  );
  const selected = useMemo(
    () => stores.find((s) => s.shopId === selectedShopId) || null,
    [stores, selectedShopId],
  );

  return (
    <div className="store-map-page">
      <Card className="store-map-page__head">
        <Title heading={5} style={{ margin: 0 }}>门店分布图</Title>
        <Text type="tertiary">
          共 {stores.length} 家门店，已定位 {located.length} 家，已设配送范围 {withRadius.length} 家。
          点击地图标记或右侧名单可定位门店；配送范围的新增与修改请到对应门店的配送设置中操作。
        </Text>
      </Card>

      <div className="store-map-page__body">
        <Card className="store-map-page__map">
          {loading ? (
            <div className="store-map-page__loading"><Spin /></div>
          ) : (
            <StoreDistributionMap
              stores={stores}
              selectedShopId={selectedShopId}
              onSelect={setSelectedShopId}
            />
          )}
        </Card>

        <Card className="store-map-page__side" title="门店名单">
          {selected && (
            <div className="store-map-page__detail">
              <div className="store-map-page__detail-name">{selected.shopTitle}</div>
              <div>{selected.address || '未填写详细地址'}</div>
              <div>{radiusText(selected)}</div>
            </div>
          )}
          <div className="store-map-page__list">
            {located.map((store) => {
              const st = statusMap[Number(store.status)] || { text: '未知状态', color: 'grey' as const };
              return (
                <button
                  type="button"
                  key={store.shopId}
                  className={`store-map-page__row${store.shopId === selectedShopId ? ' is-selected' : ''}`}
                  onClick={() => setSelectedShopId(store.shopId)}
                >
                  <span className="store-map-page__row-name">{store.shopTitle}</span>
                  <Tag color={st.color}>{st.text}</Tag>
                  <span className="store-map-page__row-sub">
                    {store.address || '未填写详细地址'} · {radiusText(store)}
                  </span>
                </button>
              );
            })}
            {!loading && located.length === 0 && <Empty description="暂无已定位门店" />}
          </div>
          {unlocated.length > 0 && (
            <div className="store-map-page__unlocated">
              <Text strong>未设置经纬度（{unlocated.length} 家，不上图）</Text>
              {unlocated.map((store) => (
                <div key={store.shopId} className="store-map-page__unlocated-row">
                  {store.shopTitle}
                  <Text type="tertiary"> · 请在门店信息中完善地址后自动上图</Text>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
