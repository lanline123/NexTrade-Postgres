import { useEffect, useRef } from "react";
import { Map, View } from "@tarojs/components";
import { Text } from "@tarojs/components";

import { getTiandituJsTk, isNativeMapProvider } from "@/utils/mapProvider";
import { gcj02ToTiandituCoordinate, tiandituToGcj02Coordinate } from "@/utils/mapCoordinate";

export interface TigMapMarker {
    id?: number;
    longitude: number;
    latitude: number;
    title?: string;
    iconPath?: string;
    width?: number;
    height?: number;
    [key: string]: any;
}

interface Props {
    mapId?: string;
    longitude?: number;
    latitude?: number;
    scale?: number;
    markers?: TigMapMarker[];
    showLocation?: boolean;
    enable3D?: boolean;
    showCompass?: boolean;
    enableOverlooking?: boolean;
    enableRotate?: boolean;
    onRegionChange?: (e: any) => void;
    onTap?: (e: any) => void;
    onMarkerTap?: (e: any) => void;
    onUpdated?: (e: any) => void;
    onError?: (e: any) => void;
}

/** 天地图托管页路径（src/static/tianditu-map.html 经 copy 发布为 /mobile/static/…）。 */
const TIANDITU_MAP_PAGE = "/mobile/static/tianditu-map.html";

/** Taro scale（3–20）收敛到天地图 zoom（3–18）。 */
const toTiandituZoom = (scale: number): number => Math.min(18, Math.max(3, Math.round(Number(scale) || 16)));

/** GCJ-02 的中心点/标记 -> 天地图托管页协议（CGCS2000 坐标）。 */
const toTiandituPayload = (longitude: number, latitude: number, scale: number, markers: TigMapMarker[]) => {
    const center = gcj02ToTiandituCoordinate({ longitude, latitude });
    return {
        lng: center.longitude,
        lat: center.latitude,
        zoom: toTiandituZoom(scale),
        markers: markers.map((marker, index) => {
            const converted = gcj02ToTiandituCoordinate({ longitude: marker.longitude, latitude: marker.latitude });
            return {
                id: marker.id ?? index,
                lng: converted.longitude,
                lat: converted.latitude,
                title: marker.title,
                iconPath: marker.iconPath,
                width: marker.width,
                height: marker.height
            };
        })
    };
};

/**
 * 天地图 H5 真渲染（iframe 嵌托管页 src/static/tianditu-map.html）。
 *
 * 原版 renderjs 在 Taro 无等价物，按组件既定路线以 iframe 嵌天地图 JS API
 * 托管页实现：中心点/标记经 gcj02ToTiandituCoordinate 转 CGCS2000 后以
 * URL 入参 + postMessage（tianditu:update）下发；托管页回传
 * ready/tap/markertap/error，tap 坐标转回 GCJ-02 后按 Taro 事件形状
 * （detail.longitude/latitude、detail.markerId）回调，与原生分支同契约。
 * iframe 以命令式 DOM 创建（本仓 uniapp-react 无原生 iframe JSX 先例），
 * 仅 H5 端走到此分支（isNativeMapProvider 在非 H5 恒真）。
 */
function TiandituMapFrame({ longitude, latitude, scale, markers, onTap, onMarkerTap, onUpdated, onError }: Required<Pick<Props, "longitude" | "latitude" | "scale" | "markers">> & Omit<Props, "longitude" | "latitude" | "scale" | "markers">) {
    const containerIdRef = useRef(`tig-tianditu-${Math.random().toString(36).slice(2, 10)}`);
    const iframeRef = useRef<HTMLIFrameElement | null>(null);
    const readyRef = useRef(false);
    const callbacksRef = useRef({ onTap, onMarkerTap, onUpdated, onError });
    callbacksRef.current = { onTap, onMarkerTap, onUpdated, onError };
    const payloadRef = useRef(toTiandituPayload(longitude, latitude, scale, markers));
    payloadRef.current = toTiandituPayload(longitude, latitude, scale, markers);

    // 挂载：创建 iframe + 监听托管页回传（仅同源、仅本 iframe 的消息）。
    useEffect(() => {
        const container = document.getElementById(containerIdRef.current);
        if (!container) return;
        const tk = getTiandituJsTk();
        const initial = payloadRef.current;
        const query = new URLSearchParams({
            tk,
            lng: String(initial.lng),
            lat: String(initial.lat),
            zoom: String(initial.zoom),
            markers: JSON.stringify(initial.markers)
        });
        const iframe = document.createElement("iframe");
        iframe.src = `${window.location.origin}${TIANDITU_MAP_PAGE}?${query.toString()}`;
        iframe.style.width = "100%";
        iframe.style.height = "100%";
        iframe.style.border = "0";
        iframe.setAttribute("title", "天地图");
        container.appendChild(iframe);
        iframeRef.current = iframe;

        const onMessage = (event: MessageEvent) => {
            if (event.origin !== window.location.origin) return;
            if (event.source !== iframe.contentWindow) return;
            const data = event.data || {};
            const callbacks = callbacksRef.current;
            if (data.type === "tianditu:ready") {
                readyRef.current = true;
                callbacks.onUpdated?.({});
            } else if (data.type === "tianditu:tap") {
                const converted = tiandituToGcj02Coordinate({ longitude: Number(data.lng), latitude: Number(data.lat) });
                callbacks.onTap?.({ detail: { longitude: converted.longitude, latitude: converted.latitude } });
            } else if (data.type === "tianditu:markertap") {
                callbacks.onMarkerTap?.({ detail: { markerId: data.markerId } });
            } else if (data.type === "tianditu:error") {
                callbacks.onError?.(new Error(String(data.message || "天地图加载失败")));
            }
        };
        window.addEventListener("message", onMessage);
        return () => {
            window.removeEventListener("message", onMessage);
            readyRef.current = false;
            iframeRef.current = null;
            iframe.remove();
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    // 属性变化：ready 后经 postMessage 同步中心点/缩放/标记。
    useEffect(() => {
        if (!readyRef.current || !iframeRef.current?.contentWindow) return;
        const payload = payloadRef.current;
        iframeRef.current.contentWindow.postMessage(
            {
                type: "tianditu:update",
                center: { lng: payload.lng, lat: payload.lat },
                zoom: payload.zoom,
                markers: payload.markers
            },
            window.location.origin
        );
    }, [longitude, latitude, scale, markers]);

    return <View id={containerIdRef.current} style={{ width: "100%", height: "300px", background: "#f5f5f5" }} />;
}

/**
 * 地图（自旧 tig-map.vue 对位移植，渲染层补齐）。
 * 旧版双分支：原生 <map> / 天地图 renderjs。重构版：
 * - isNativeMapProvider() 为真（小程序恒真；H5 未选天地图或未配 JS TK 时）
 *   → Taro <Map>，props 与旧版一一透传（enable3D/showCompass 等原生能力一致）；
 * - 天地图 H5 分支 → TiandituMapFrame（iframe 嵌天地图 JS API 托管页真渲染，
 *   协议见 src/static/tianditu-map.html 头注）。
 */
export default function TigMap({
    longitude = 0,
    latitude = 0,
    scale = 16,
    markers = [],
    showLocation = false,
    enable3D = false,
    showCompass = false,
    enableOverlooking = false,
    enableRotate = false,
    onRegionChange,
    onTap,
    onMarkerTap,
    onUpdated,
    onError
}: Props) {
    if (!isNativeMapProvider()) {
        if (!getTiandituJsTk()) {
            // 防御分支：判定与取 TK 口径一致时不可达；万一配置竞态，
            // 保留信息占位而不是嵌一个必然失败的 iframe。
            return (
                <View className="tig-map-fallback" style={{ width: "100%", height: "300px", display: "flex", alignItems: "center", justifyContent: "center", background: "#f5f5f5" }}>
                    <Text style={{ color: "#999", fontSize: "12px" }}>
                        天地图未配置 JS TK（中心 {longitude},{latitude}，标记 {markers.length}）
                    </Text>
                </View>
            );
        }
        return (
            <TiandituMapFrame
                longitude={longitude}
                latitude={latitude}
                scale={scale}
                markers={markers}
                onTap={onTap}
                onMarkerTap={onMarkerTap}
                onUpdated={onUpdated}
                onError={onError}
            />
        );
    }
    return (
        <Map
            longitude={longitude}
            latitude={latitude}
            scale={scale}
            markers={markers as any}
            showLocation={showLocation}
            enable3D={enable3D}
            showCompass={showCompass}
            enableOverlooking={enableOverlooking}
            enableRotate={enableRotate}
            style={{ width: "100%", height: "300px" }}
            onRegionChange={onRegionChange}
            onTap={onTap}
            onMarkerTap={onMarkerTap}
            onUpdated={onUpdated}
            onError={onError || (() => {})}
        />
    );
}
