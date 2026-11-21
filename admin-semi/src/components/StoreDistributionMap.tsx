import { useEffect, useMemo, useRef, useState } from 'react';
import { Spin, Typography } from '@douyinfe/semi-ui';
import { IconMapPin } from '@douyinfe/semi-icons';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import { gcj02ToWgs84 } from './mapShared';

const { Text } = Typography;

interface MapSettings {
  mapProvider?: string;
  amapJsKey?: string;
  amapJsSecret?: string;
  tiandituJsTk?: string;
}

/** 分布图单店入参（与后端 StoreMapVO 对齐的子集）。 */
export interface StoreMapPoint {
  shopId: number;
  shopTitle: string;
  longitude?: number | null;
  latitude?: number | null;
  deliveryRadius?: number | null;
  deliveryEnabled?: number | null;
}

type LngLat = [number, number];

interface OverlayEntry {
  circle?: any;
  lnglat: LngLat;
}

function hasUsableKey(value?: string) {
  const key = String(value || '').trim();
  return Boolean(key && key !== '0');
}

function isLocated(store: StoreMapPoint): store is StoreMapPoint & { longitude: number; latitude: number } {
  const lng = Number(store.longitude);
  const lat = Number(store.latitude);
  return Number.isFinite(lng) && Number.isFinite(lat) && Boolean(lng) && Boolean(lat);
}

const NORMAL_STYLE = { strokeColor: '#165dff', strokeWeight: 2, fillColor: '#165dff' };
const SELECTED_STYLE = { strokeColor: '#f77234', strokeWeight: 3, fillColor: '#f77234' };

/**
 * 门店分布图（我方新增，论坛帖 587859 余子项）：全部已定位门店打点 +
 * 各自同城配送范围圈叠加。地图装载/双分支（高德/天地图）/坐标口径与
 * 单店 `DeliveryRadiusMap` 完全一致：门店坐标按 GCJ-02 存储，高德直接用、
 * 天地图入图前经 `gcj02ToWgs84` 转换；本组件只读展示，不提供圈编辑。
 */
export default function StoreDistributionMap({
  stores,
  selectedShopId,
  onSelect,
}: {
  stores: StoreMapPoint[];
  selectedShopId?: number | null;
  onSelect?: (shopId: number) => void;
}) {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const mapRef = useRef<any>(null);
  const overlaysRef = useRef<Map<number, OverlayEntry>>(new Map());
  const providerRef = useRef<'amap' | 'tianditu'>('amap');
  const onSelectRef = useRef(onSelect);
  onSelectRef.current = onSelect;
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  const located = useMemo(() => stores.filter(isLocated), [stores]);
  // 门店集合签名（ID + 坐标 + 半径），变化时整体重建覆盖物。
  const signature = useMemo(
    () => located.map((s) => `${s.shopId}:${s.longitude},${s.latitude}:${s.deliveryRadius || 0}`).join('|'),
    [located],
  );

  useEffect(() => {
    let disposed = false;

    const loadAmap = (key: string, secret: string) => new Promise<any>((resolve, reject) => {
      const existing = (window as any).AMap;
      if (existing) {
        resolve(existing);
        return;
      }
      if (secret) {
        (window as any)._AMapSecurityConfig = { securityJsCode: secret };
      }
      const script = document.createElement('script');
      script.src = `https://webapi.amap.com/maps?v=2.0&key=${encodeURIComponent(key)}`;
      script.async = true;
      script.onload = () => resolve((window as any).AMap);
      script.onerror = () => reject(new Error(t('地图加载失败')));
      document.head.appendChild(script);
    });

    const loadTianditu = (tk: string) => new Promise<any>((resolve, reject) => {
      const existing = (window as any).T;
      if (existing?.Map) {
        resolve(existing);
        return;
      }
      const script = document.createElement('script');
      script.src = `https://api.tianditu.gov.cn/api?v=4.0&tk=${encodeURIComponent(tk)}`;
      script.async = true;
      script.onload = () => {
        const T = (window as any).T;
        if (T?.Map) resolve(T);
        else reject(new Error(t('地图加载失败')));
      };
      script.onerror = () => reject(new Error(t('地图加载失败')));
      document.head.appendChild(script);
    });

    const initAmap = async (settings: MapSettings) => {
      const AMap = await loadAmap(String(settings?.amapJsKey), String(settings?.amapJsSecret || ''));
      if (disposed || !containerRef.current) return;
      const map = new AMap.Map(containerRef.current, { zoom: 11, viewMode: '2D' });
      mapRef.current = map;
      const overlays: any[] = [];
      located.forEach((store) => {
        const position: LngLat = [Number(store.longitude), Number(store.latitude)];
        const marker = new AMap.Marker({ position, title: store.shopTitle });
        marker.on('click', () => onSelectRef.current?.(store.shopId));
        map.add(marker);
        overlays.push(marker);
        let circle: any;
        const radius = Number(store.deliveryRadius) || 0;
        // 圈只画「已配置且同城配送开启」的有效范围（与买家端范围判定同口径）。
        if (radius > 0 && store.deliveryEnabled !== 0) {
          circle = new AMap.Circle({
            center: position,
            radius,
            strokeColor: NORMAL_STYLE.strokeColor,
            strokeWeight: NORMAL_STYLE.strokeWeight,
            strokeOpacity: 0.8,
            strokeStyle: 'dashed',
            fillColor: NORMAL_STYLE.fillColor,
            fillOpacity: 0.08,
          });
          map.add(circle);
          overlays.push(circle);
        }
        overlaysRef.current.set(store.shopId, { circle, lnglat: position });
      });
      if (overlays.length) map.setFitView(overlays);
    };

    const initTianditu = async (settings: MapSettings) => {
      const T = await loadTianditu(String(settings?.tiandituJsTk));
      if (disposed || !containerRef.current) return;
      const map = new T.Map(containerRef.current);
      mapRef.current = map;
      map.enableScrollWheelZoom?.();
      const points: any[] = [];
      located.forEach((store) => {
        const wgs = gcj02ToWgs84(Number(store.longitude), Number(store.latitude));
        const lnglat = new T.LngLat(wgs[0], wgs[1]);
        points.push(lnglat);
        const marker = new T.Marker(lnglat);
        marker.addEventListener?.('click', () => onSelectRef.current?.(store.shopId));
        map.addOverLay(marker);
        let circle: any;
        const radius = Number(store.deliveryRadius) || 0;
        if (radius > 0 && store.deliveryEnabled !== 0) {
          circle = new T.Circle(lnglat, radius, {
            color: NORMAL_STYLE.strokeColor,
            weight: NORMAL_STYLE.strokeWeight,
            opacity: 0.8,
            lineStyle: 'dashed',
            fillColor: NORMAL_STYLE.fillColor,
            fillOpacity: 0.08,
          });
          map.addOverLay(circle);
        }
        overlaysRef.current.set(store.shopId, { circle, lnglat: wgs });
      });
      if (points.length === 1) {
        map.centerAndZoom(points[0], 13);
      } else if (points.length > 1) {
        try {
          map.setViewport(points);
        } catch {
          map.centerAndZoom(points[0], 11);
        }
      }
    };

    const init = async () => {
      setLoading(true);
      setError('');
      overlaysRef.current = new Map();
      try {
        if (!located.length) {
          setError(t('暂无已设置经纬度的门店，请先在门店信息中完善地址'));
          return;
        }
        const settings = await apiGet<MapSettings>('/setting/config/mapSettings').catch(() => null);
        if (disposed) return;
        const isTianditu = settings?.mapProvider === 'tianditu';
        providerRef.current = isTianditu ? 'tianditu' : 'amap';
        if (isTianditu) {
          if (!hasUsableKey(settings?.tiandituJsTk)) {
            setError(t('未配置天地图 JS TK，请先在「地图设置」中配置'));
            return;
          }
          await initTianditu(settings || {});
        } else {
          if (!hasUsableKey(settings?.amapJsKey)) {
            setError(t('未配置高德地图 JS Key，请先在「地图设置」中配置'));
            return;
          }
          await initAmap(settings || {});
        }
      } catch (err) {
        if (!disposed) {
          setError(err instanceof Error ? err.message : t('地图加载失败'));
        }
      } finally {
        if (!disposed) setLoading(false);
      }
    };

    void init();
    return () => {
      disposed = true;
      overlaysRef.current = new Map();
      mapRef.current?.destroy?.();
      mapRef.current?.clearOverLays?.();
      mapRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);

  // 选中态：高亮对应配送圈并把地图中心移到该门店。
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;
    overlaysRef.current.forEach((entry, shopId) => {
      const selected = shopId === selectedShopId;
      const style = selected ? SELECTED_STYLE : NORMAL_STYLE;
      entry.circle?.setOptions?.({
        strokeColor: style.strokeColor,
        strokeWeight: style.strokeWeight,
        fillColor: style.fillColor,
      });
      entry.circle?.setColor?.(style.strokeColor);
      entry.circle?.setFillColor?.(style.fillColor);
      entry.circle?.setWeight?.(style.strokeWeight);
    });
    if (selectedShopId != null) {
      const entry = overlaysRef.current.get(selectedShopId);
      if (entry) {
        if (providerRef.current === 'tianditu') {
          const T = (window as any).T;
          if (T?.LngLat) map.panTo?.(new T.LngLat(entry.lnglat[0], entry.lnglat[1]));
        } else {
          map.setCenter?.(entry.lnglat);
        }
      }
    }
  }, [selectedShopId, signature]);

  return (
    <div className="store-distribution-map">
      {loading && (
        <div className="store-distribution-map__state">
          <Spin />
        </div>
      )}
      {!loading && error && (
        <div className="store-distribution-map__state">
          <span className="store-distribution-map__icon"><IconMapPin /></span>
          <Text type="tertiary">{error}</Text>
        </div>
      )}
      <div ref={containerRef} className="store-distribution-map__canvas" style={{ display: error ? 'none' : 'block' }} />
    </div>
  );
}
