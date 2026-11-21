import { useEffect, useRef, useState } from 'react';
import { Button, Input, Spin, Toast, Typography } from '@douyinfe/semi-ui';
import { IconMapPin, IconSearch } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import {
  fetchMapSettings,
  gcj02ToWgs84,
  hasUsableKey,
  isValidLngLat,
  loadAmapSdk,
  loadTiandituSdk,
  wgs84ToGcj02,
  type LngLat,
  type MapSettings,
} from './mapShared';

const { Text } = Typography;

/** 无已存坐标时的默认视野中心（北京，GCJ-02）。 */
const DEFAULT_CENTER: LngLat = [116.397428, 39.90923];

function pointKey(p?: LngLat | null) {
  return isValidLngLat(p) ? `${p[0]},${p[1]}` : '';
}

/** 天地图 Geocoder 回调结果形状在版本间不稳定，统一防御式归一为经纬度。 */
function normalizeTiandituPoint(result: any): LngLat | null {
  if (!result) return null;
  const ll = result.getLngLat?.() ?? result.lnglat ?? result.location ?? result;
  const lng = Number(ll?.getLng?.() ?? ll?.lng ?? ll?.[0]);
  const lat = Number(ll?.getLat?.() ?? ll?.lat ?? ll?.[1]);
  return Number.isFinite(lng) && Number.isFinite(lat) && (lng !== 0 || lat !== 0)
    ? [lng, lat]
    : null;
}

function normalizeTiandituAddress(result: any): string | undefined {
  if (!result) return undefined;
  const addr =
    result.getAddress?.() ??
    result.formattedAddress ??
    result.formatted_address ??
    result.address ??
    result.result?.formatted_address;
  return typeof addr === 'string' && addr.trim() ? addr.trim() : undefined;
}

/**
 * 地图选点组件（门店/自提点表单用）：搜索定位 + 地图点选/拖拽标记落点，
 * 逆地理回填地址。坐标口径与 DeliveryRadiusMap 完全一致——
 * 对外一律 GCJ-02（库存口径），天地图分支内部做 WGS84 转换；
 * SDK 加载与设置读取共用 mapShared，不另起一套。
 */
export default function MapLocationPicker({
  value,
  defaultKeyword,
  onChange,
}: {
  /** 当前坐标（GCJ-02）；为空表示尚未选点。 */
  value?: LngLat | null;
  /** 搜索框初始值（一般传表单里已填的详细地址）。 */
  defaultKeyword?: string;
  /** 选点回调：point 为 GCJ-02；address 为逆地理地址（取不到时不回填）。 */
  onChange: (point: LngLat, address?: string) => void;
}) {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const mapRef = useRef<any>(null);
  const markerRef = useRef<any>(null);
  const sdkRef = useRef<any>(null);
  const providerRef = useRef<'amap' | 'tianditu' | null>(null);
  const geocoderRef = useRef<any>(null);
  const readyRef = useRef(false);
  const lastEmittedRef = useRef('');
  const valueRef = useRef<LngLat | null | undefined>(value);
  valueRef.current = value;
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [keyword, setKeyword] = useState(defaultKeyword || '');
  const [searching, setSearching] = useState(false);
  const [pickedAddress, setPickedAddress] = useState('');
  const searchRef = useRef<(kw: string) => void>(() => undefined);

  useEffect(() => {
    let disposed = false;

    const emit = (point: LngLat, address?: string) => {
      lastEmittedRef.current = pointKey(point);
      if (address) setPickedAddress(address);
      onChangeRef.current(point, address);
    };

    /** 在地图上放置/移动标记；display 为当前地图坐标系下的点。 */
    const placeMarker = (display: LngLat) => {
      const sdk = sdkRef.current;
      const map = mapRef.current;
      if (!sdk || !map) return;
      if (providerRef.current === 'amap') {
        if (!markerRef.current) {
          markerRef.current = new sdk.Marker({ draggable: true, cursor: 'move' });
          markerRef.current.on('dragend', () => {
            const pos = markerRef.current.getPosition();
            if (pos) pickAmap([pos.getLng(), pos.getLat()]);
          });
          map.add(markerRef.current);
        }
        markerRef.current.setPosition(display);
      } else {
        const lngLat = new sdk.LngLat(display[0], display[1]);
        if (!markerRef.current) {
          markerRef.current = new sdk.Marker(lngLat);
          markerRef.current.enableDragging?.();
          markerRef.current.addEventListener?.('dragend', () => {
            const pos = markerRef.current.getLngLat?.();
            if (pos) pickTianditu([pos.getLng(), pos.getLat()]);
          });
          map.addOverLay(markerRef.current);
        } else {
          markerRef.current.setLngLat?.(lngLat);
        }
      }
    };

    /** 高德分支选点：入参即 GCJ-02，逆地理后回填。 */
    const pickAmap = (gcj: LngLat) => {
      placeMarker(gcj);
      const geocoder = geocoderRef.current;
      if (!geocoder?.getAddress) {
        emit(gcj);
        return;
      }
      try {
        geocoder.getAddress(gcj, (status: string, result: any) => {
          const address =
            status === 'complete' ? result?.regeocode?.formattedAddress : undefined;
          emit(gcj, typeof address === 'string' && address ? address : undefined);
        });
      } catch {
        emit(gcj);
      }
    };

    /** 天地图分支选点：入参为地图 WGS84 点，出参转 GCJ-02。 */
    const pickTianditu = (wgs: LngLat) => {
      placeMarker(wgs);
      const gcj = wgs84ToGcj02(wgs[0], wgs[1]);
      const sdk = sdkRef.current;
      try {
        const geocoder = geocoderRef.current ?? (sdk?.Geocoder ? new sdk.Geocoder() : null);
        geocoderRef.current = geocoder;
        if (geocoder?.getLocation) {
          geocoder.getLocation(new sdk.LngLat(wgs[0], wgs[1]), (result: any) => {
            emit(gcj, normalizeTiandituAddress(result));
          });
          return;
        }
      } catch {
        // 逆地理不可用不阻断落点：坐标照常回填，地址保留表单原值。
      }
      emit(gcj);
    };

    const initAmap = async (settings: MapSettings | null) => {
      const AMap = await loadAmapSdk(
        String(settings?.amapJsKey),
        String(settings?.amapJsSecret || ''),
      );
      if (disposed || !containerRef.current) return;
      sdkRef.current = AMap;
      providerRef.current = 'amap';
      const start = isValidLngLat(valueRef.current) ? valueRef.current : DEFAULT_CENTER;
      const map = new AMap.Map(containerRef.current, {
        center: start,
        zoom: isValidLngLat(valueRef.current) ? 15 : 11,
        viewMode: '2D',
      });
      mapRef.current = map;
      try {
        geocoderRef.current = AMap.Geocoder ? new AMap.Geocoder({}) : null;
      } catch {
        geocoderRef.current = null;
      }
      map.on('click', (e: any) => {
        if (e?.lnglat) pickAmap([e.lnglat.getLng(), e.lnglat.getLat()]);
      });
      if (isValidLngLat(valueRef.current)) placeMarker(valueRef.current);
      readyRef.current = true;
    };

    const initTianditu = async (settings: MapSettings | null) => {
      const T = await loadTiandituSdk(String(settings?.tiandituJsTk));
      if (disposed || !containerRef.current) return;
      sdkRef.current = T;
      providerRef.current = 'tianditu';
      const startGcj = isValidLngLat(valueRef.current) ? valueRef.current : DEFAULT_CENTER;
      const startWgs = gcj02ToWgs84(startGcj[0], startGcj[1]);
      const map = new T.Map(containerRef.current);
      map.centerAndZoom(
        new T.LngLat(startWgs[0], startWgs[1]),
        isValidLngLat(valueRef.current) ? 15 : 11,
      );
      map.enableScrollWheelZoom?.();
      mapRef.current = map;
      map.addEventListener?.('click', (e: any) => {
        const ll = e?.lnglat;
        if (ll) pickTianditu([ll.getLng(), ll.getLat()]);
      });
      if (isValidLngLat(valueRef.current)) placeMarker(startWgs);
      readyRef.current = true;
    };

    const init = async () => {
      setLoading(true);
      setError('');
      try {
        const settings = await fetchMapSettings();
        if (disposed) return;
        if (settings?.mapProvider === 'tianditu') {
          if (!hasUsableKey(settings?.tiandituJsTk)) {
            setError(t('未配置天地图 JS TK，请先在「地图设置」中配置'));
            return;
          }
          await initTianditu(settings);
        } else {
          if (!hasUsableKey(settings?.amapJsKey)) {
            setError(t('未配置高德地图 JS Key，请先在「地图设置」中配置'));
            return;
          }
          await initAmap(settings);
        }
      } catch (err) {
        if (!disposed) {
          setError(err instanceof Error ? err.message : t('地图加载失败'));
        }
      } finally {
        if (!disposed) setLoading(false);
      }
    };

    // 供搜索按钮使用的内部句柄（与初始化同闭包，避免重复实现两分支搜索）。
    searchRef.current = (kw: string) => {
      const sdk = sdkRef.current;
      const map = mapRef.current;
      if (!sdk || !map || !readyRef.current) return;
      setSearching(true);
      if (providerRef.current === 'amap') {
        const geocoder = geocoderRef.current;
        if (!geocoder?.getLocation) {
          setSearching(false);
          Toast.warning({ content: t('地图搜索不可用，请直接在地图上点选') });
          return;
        }
        geocoder.getLocation(kw, (status: string, result: any) => {
          setSearching(false);
          const first = status === 'complete' ? result?.geocodes?.[0] : null;
          const loc = first?.location;
          if (loc) {
            const gcj: LngLat = [loc.getLng(), loc.getLat()];
            map.setZoomAndCenter?.(15, gcj);
            pickAmap(gcj);
          } else {
            Toast.warning({ content: t('未找到该地址，请在地图上点选') });
          }
        });
      } else {
        try {
          const geocoder = geocoderRef.current ?? (sdk.Geocoder ? new sdk.Geocoder() : null);
          geocoderRef.current = geocoder;
          if (!geocoder?.getPoint) {
            setSearching(false);
            Toast.warning({ content: t('地图搜索不可用，请直接在地图上点选') });
            return;
          }
          geocoder.getPoint(kw, (result: any) => {
            setSearching(false);
            const wgs = normalizeTiandituPoint(result);
            if (wgs) {
              map.centerAndZoom?.(new sdk.LngLat(wgs[0], wgs[1]), 15);
              pickTianditu(wgs);
            } else {
              Toast.warning({ content: t('未找到该地址，请在地图上点选') });
            }
          });
        } catch {
          setSearching(false);
          Toast.warning({ content: t('未找到该地址，请在地图上点选') });
        }
      }
    };

    void init();
    return () => {
      disposed = true;
      readyRef.current = false;
      markerRef.current = null;
      geocoderRef.current = null;
      mapRef.current?.destroy?.();
      mapRef.current?.clearOverLays?.();
      mapRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 表单侧手工改经纬度时，地图标记/视野跟随（不回调用 onChange，避免循环）。
  useEffect(() => {
    if (!readyRef.current || !mapRef.current || !sdkRef.current) return;
    if (!isValidLngLat(value)) return;
    if (pointKey(value) === lastEmittedRef.current) return;
    lastEmittedRef.current = pointKey(value);
    const display =
      providerRef.current === 'tianditu' ? gcj02ToWgs84(value[0], value[1]) : value;
    // 与 placeMarker 同逻辑的轻量同步：重建标记位置并居中。
    const sdk = sdkRef.current;
    const map = mapRef.current;
    if (providerRef.current === 'amap') {
      if (markerRef.current) {
        markerRef.current.setPosition(display);
        map.setCenter?.(display);
      }
    } else if (markerRef.current) {
      markerRef.current.setLngLat?.(new sdk.LngLat(display[0], display[1]));
      map.panTo?.(new sdk.LngLat(display[0], display[1]));
    }
  }, [value]);

  const handleSearch = () => {
    const kw = keyword.trim();
    if (!kw) {
      Toast.warning({ content: t('请先输入地址再搜索') });
      return;
    }
    searchRef.current(kw);
  };

  return (
    <div className="map-location-picker">
      <div className="map-location-picker__toolbar">
        <Input
          value={keyword}
          onChange={setKeyword}
          placeholder={t('输入地址搜索定位')}
          showClear
          onEnterPress={handleSearch}
          style={{ flex: 1 }}
        />
        <Button icon={<IconSearch />} loading={searching} onClick={handleSearch}>
          {t('搜索定位')}
        </Button>
      </div>
      <div className="map-location-picker__map">
        {loading && (
          <div className="map-location-picker__state">
            <Spin />
          </div>
        )}
        {!loading && error && (
          <div className="map-location-picker__state">
            <span className="map-location-picker__icon">
              <IconMapPin />
            </span>
            <Text type="tertiary">{error}</Text>
          </div>
        )}
        <div
          ref={containerRef}
          className="map-location-picker__canvas"
          style={{ display: error ? 'none' : 'block' }}
        />
      </div>
      <div className="map-location-picker__hint">
        <Text type="tertiary" size="small">
          {pickedAddress
            ? `${t('已选位置')}：${pickedAddress}`
            : t('点击地图或拖动标记选点，坐标自动回填经纬度，地址同步回填详细地址')}
        </Text>
      </div>
    </div>
  );
}
