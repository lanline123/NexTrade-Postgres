import { useEffect, useRef, useState } from 'react';
import { Spin, Typography } from '@douyinfe/semi-ui';
import { IconMapPin } from '@douyinfe/semi-icons';
import { apiGet } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

interface MapSettings {
  mapProvider?: string;
  amapJsKey?: string;
  amapJsSecret?: string;
  tiandituJsTk?: string;
}

type LngLat = [number, number];

function hasUsableKey(value?: string) {
  const key = String(value || '').trim();
  return Boolean(key && key !== '0');
}

// ——— GCJ-02 <-> WGS84（天地图 CGCS2000 按 WGS84 处理）———
// 门店经纬度按高德口径存 GCJ-02；天地图分支进出地图前转换。
// admin 无 gcoord 依赖，此处内嵌标准算法（ Krasovsky 椭球参数 + 迭代反解，
// 与 H5 侧 gcoord 的 GCJ02<->WGS84 口径一致，误差米级，配送圈展示足够）。
const GCJ_A = 6378245.0;
const GCJ_EE = 0.00669342162296594323;

function outOfChina(lng: number, lat: number) {
  return lng < 72.004 || lng > 137.8347 || lat < 0.8293 || lat > 55.8271;
}

function transformLat(x: number, y: number) {
  let ret = -100.0 + 2.0 * x + 3.0 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * Math.sqrt(Math.abs(x));
  ret += ((20.0 * Math.sin(6.0 * x * Math.PI) + 20.0 * Math.sin(2.0 * x * Math.PI)) * 2.0) / 3.0;
  ret += ((20.0 * Math.sin(y * Math.PI) + 40.0 * Math.sin((y / 3.0) * Math.PI)) * 2.0) / 3.0;
  ret += ((160.0 * Math.sin((y / 12.0) * Math.PI) + 320 * Math.sin((y * Math.PI) / 30.0)) * 2.0) / 3.0;
  return ret;
}

function transformLng(x: number, y: number) {
  let ret = 300.0 + x + 2.0 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * Math.sqrt(Math.abs(x));
  ret += ((20.0 * Math.sin(6.0 * x * Math.PI) + 20.0 * Math.sin(2.0 * x * Math.PI)) * 2.0) / 3.0;
  ret += ((20.0 * Math.sin(x * Math.PI) + 40.0 * Math.sin((x / 3.0) * Math.PI)) * 2.0) / 3.0;
  ret += ((150.0 * Math.sin((x / 12.0) * Math.PI) + 300.0 * Math.sin((x / 30.0) * Math.PI)) * 2.0) / 3.0;
  return ret;
}

function wgs84ToGcj02(lng: number, lat: number): LngLat {
  if (outOfChina(lng, lat)) return [lng, lat];
  let dLat = transformLat(lng - 105.0, lat - 35.0);
  let dLng = transformLng(lng - 105.0, lat - 35.0);
  const radLat = (lat / 180.0) * Math.PI;
  let magic = Math.sin(radLat);
  magic = 1 - GCJ_EE * magic * magic;
  const sqrtMagic = Math.sqrt(magic);
  dLat = (dLat * 180.0) / (((GCJ_A * (1 - GCJ_EE)) / (magic * sqrtMagic)) * Math.PI);
  dLng = (dLng * 180.0) / ((GCJ_A / sqrtMagic) * Math.cos(radLat) * Math.PI);
  return [lng + dLng, lat + dLat];
}

function gcj02ToWgs84(lng: number, lat: number): LngLat {
  if (outOfChina(lng, lat)) return [lng, lat];
  // 迭代反解：以 GCJ 点为初值，两轮逼近把正向变换残差压到米级以内。
  let wgs: LngLat = [lng, lat];
  for (let i = 0; i < 2; i += 1) {
    const gcj = wgs84ToGcj02(wgs[0], wgs[1]);
    wgs = [lng - (gcj[0] - wgs[0]), lat - (gcj[1] - wgs[1])];
  }
  return wgs;
}

/** 天地图圆心（WGS84）正东方向 radius 米处的经纬度（拖拽手柄落点）。 */
function eastEdgePoint(centerWgs: LngLat, radiusMeters: number): LngLat {
  const metersPerDegreeLng = 111320 * Math.cos((centerWgs[1] * Math.PI) / 180);
  const delta = metersPerDegreeLng > 1 ? radiusMeters / metersPerDegreeLng : 0;
  return [centerWgs[0] + delta, centerWgs[1]];
}

export default function DeliveryRadiusMap({
  center,
  radius,
  onChangeRadius,
}: {
  center?: LngLat;
  radius: number;
  onChangeRadius: (radius: number) => void;
}) {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const mapRef = useRef<any>(null);
  const circleRef = useRef<any>(null);
  const editorRef = useRef<any>(null);
  // 天地图分支专有：可拖拽的半径手柄与 WGS84 圆心（高德分支不写这两个 ref）。
  const handleRef = useRef<any>(null);
  const tiandituCenterRef = useRef<LngLat | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

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
      script.src = `https://webapi.amap.com/maps?v=2.0&key=${encodeURIComponent(key)}&plugin=AMap.CircleEditor`;
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

    const initAmap = async (settings: MapSettings, validCenter: LngLat) => {
      const AMap = await loadAmap(String(settings?.amapJsKey), String(settings?.amapJsSecret || ''));
      if (disposed || !containerRef.current) return;
      const map = new AMap.Map(containerRef.current, {
        center: validCenter,
        zoom: 14,
        viewMode: '2D',
      });
      mapRef.current = map;
      const circle = new AMap.Circle({
        center: validCenter,
        radius: Math.max(Number(radius) || 1000, 1),
        strokeColor: '#165dff',
        strokeWeight: 2,
        strokeOpacity: 0.8,
        strokeStyle: 'dashed',
        fillColor: '#165dff',
        fillOpacity: 0.08,
      });
      map.add(circle);
      circleRef.current = circle;
      map.setFitView([circle]);
      const editor = new AMap.CircleEditor(map, circle);
      editor.open();
      const syncRadius = () => {
        const next = Math.round(Number(circle.getRadius()) || 0);
        if (next > 0) onChangeRadius(next);
      };
      editor.on('adjust', syncRadius);
      editor.on('end', syncRadius);
      editorRef.current = editor;
    };

    const initTianditu = async (settings: MapSettings, validCenter: LngLat) => {
      const T = await loadTianditu(String(settings?.tiandituJsTk));
      if (disposed || !containerRef.current) return;
      // 门店坐标是 GCJ-02（高德口径），天地图用 CGCS2000：入图先转换。
      const centerWgs = gcj02ToWgs84(validCenter[0], validCenter[1]);
      tiandituCenterRef.current = centerWgs;
      const centerLngLat = new T.LngLat(centerWgs[0], centerWgs[1]);
      const map = new T.Map(containerRef.current);
      map.centerAndZoom(centerLngLat, 14);
      map.enableScrollWheelZoom?.();
      mapRef.current = map;
      const initialRadius = Math.max(Number(radius) || 1000, 1);
      const circle = new T.Circle(centerLngLat, initialRadius, {
        color: '#165dff',
        weight: 2,
        opacity: 0.8,
        lineStyle: 'dashed',
        fillColor: '#165dff',
        fillOpacity: 0.08,
      });
      map.addOverLay(circle);
      circleRef.current = circle;
      try {
        const bounds = circle.getBounds?.();
        if (bounds) map.setViewport([bounds.getNorthEast(), bounds.getSouthWest()]);
      } catch {
        // 视野适配失败不阻断渲染，圆心定位已生效。
      }
      // 天地图无 CircleEditor 等价物：以圆周正东点放可拖拽手柄，
      // 拖拽距离经 map.getDistance 换算回半径（与高德拖圆点同交互）。
      const edge = eastEdgePoint(centerWgs, initialRadius);
      const handle = new T.Marker(new T.LngLat(edge[0], edge[1]));
      handle.enableDragging?.();
      map.addOverLay(handle);
      handleRef.current = handle;
      const syncFromHandle = () => {
        const pos = handle.getLngLat?.();
        if (!pos) return;
        const next = Math.round(Number(map.getDistance(centerLngLat, pos)) || 0);
        if (next > 0) {
          circle.setRadius(next);
          onChangeRadius(next);
        }
      };
      handle.addEventListener?.('dragging', syncFromHandle);
      handle.addEventListener?.('dragend', syncFromHandle);
    };

    const init = async () => {
      setLoading(true);
      setError('');
      try {
        const settings = await apiGet<MapSettings>('/setting/config/mapSettings').catch(() => null);
        if (disposed) return;
        const isTianditu = settings?.mapProvider === 'tianditu';
        if (isTianditu) {
          if (!hasUsableKey(settings?.tiandituJsTk)) {
            setError(t('未配置天地图 JS TK，请先在「地图设置」中配置'));
            return;
          }
        } else if (!hasUsableKey(settings?.amapJsKey)) {
          setError(t('未配置高德地图 JS Key，请先在「地图设置」中配置'));
          return;
        }
        if (!center || !Number.isFinite(center[0]) || !Number.isFinite(center[1]) || (!center[0] && !center[1])) {
          setError(t('门店未设置经纬度，请先在门店信息中完善地址'));
          return;
        }
        if (isTianditu) {
          await initTianditu(settings || {}, center);
        } else {
          await initAmap(settings || {}, center);
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
      editorRef.current?.close?.();
      editorRef.current = null;
      circleRef.current = null;
      handleRef.current = null;
      tiandituCenterRef.current = null;
      mapRef.current?.destroy?.();
      mapRef.current?.clearOverLays?.();
      mapRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [center?.[0], center?.[1]]);

  useEffect(() => {
    if (circleRef.current && radius > 0) {
      circleRef.current.setRadius(radius);
    }
    // 天地图分支：半径经数字输入改动时，手柄同步移到新圆周点。
    if (handleRef.current && tiandituCenterRef.current && radius > 0) {
      const edge = eastEdgePoint(tiandituCenterRef.current, radius);
      const T = (window as any).T;
      if (T?.LngLat) {
        handleRef.current.setLngLat?.(new T.LngLat(edge[0], edge[1]));
      }
    }
  }, [radius]);

  return (
    <div className="delivery-radius-map">
      {loading && (
        <div className="delivery-radius-map__state">
          <Spin />
        </div>
      )}
      {!loading && error && (
        <div className="delivery-radius-map__state">
          <span className="delivery-radius-map__icon"><IconMapPin /></span>
          <Text type="tertiary">{error}</Text>
        </div>
      )}
      <div ref={containerRef} className="delivery-radius-map__canvas" style={{ display: error ? 'none' : 'block' }} />
    </div>
  );
}
