import { Map, View } from "@tarojs/components";
import { Text } from "@tarojs/components";

import { isNativeMapProvider } from "@/utils/mapProvider";

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

/**
 * 地图（自旧 tig-map.vue 对位移植，渲染层补齐）。
 * 旧版双分支：原生 <map> / 天地图 renderjs。重构版：
 * - isNativeMapProvider() 为真（小程序恒真；H5 未配天地图时）→ Taro <Map>，
 *   props 与旧版一一透传（enable3D/showCompass 等原生能力一致）；
 * - 天地图 H5 分支依赖 renderjs 注入天地图 JS API，Taro 无 renderjs 等价物，
 *   此处渲染降级占位并保留中心点/标记数信息——该分支无法等价对齐，
 *   需天地图 H5 时应以 iframe 嵌天地图 JS API 页另行实现（单列未对齐项）。
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
        return (
            <View className="tig-map-fallback" style={{ width: "100%", height: "300px", display: "flex", alignItems: "center", justifyContent: "center", background: "#f5f5f5" }}>
                <Text style={{ color: "#999", fontSize: "12px" }}>
                    天地图 H5 渲染未接入（中心 {longitude},{latitude}，标记 {markers.length}）
                </Text>
            </View>
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
