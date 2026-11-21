import { useEffect, useRef, useState } from 'react';
import { Spin, Typography } from '@douyinfe/semi-ui';
import { IconMapPin } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import {
  fetchMapSettings,
  gcj02ToWgs84,
  hasUsableKey,
  loadAmapSdk,
  loadTiandituSdk,
  type LngLat,
  type MapSettings,
} from './mapShared';

const { Text } = Typography;

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

    const initAmap = async (settings: MapSettings, validCenter: LngLat) => {
      const AMap = await loadAmapSdk(String(settings?.amapJsKey), String(settings?.amapJsSecret || ''));
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
      const T = await loadTiandituSdk(String(settings?.tiandituJsTk));
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
        const settings = await fetchMapSettings();
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
