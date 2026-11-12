interface StoreNavigationDestination {
    longitude: string | number;
    latitude: string | number;
    name?: string;
    address?: string;
}

interface TiandituShareResponse {
    ok: boolean;
    json: () => Promise<any>;
}

type TiandituShareRequest = (url: string, options: { method: string; headers: Record<string, string>; body: string }) => Promise<TiandituShareResponse>;

const TIANDITU_SHARE_API = "https://map.tianditu.gov.cn/api/map/share";
const TIANDITU_SHARE_PAGE = "https://map.tianditu.gov.cn/share";

export const buildTiandituShareState = (destination: StoreNavigationDestination) => {
    const longitude = Number(destination.longitude);
    const latitude = Number(destination.latitude);
    const name = destination.name || "目的地";
    const address = destination.address || "";
    const lonlat = `${longitude},${latitude}`;

    return {
        popupInfo: {
            show: true,
            propertyShow: false,
            view: "poi",
            lngLat: [longitude, latitude],
            data: {
                featureType: "0",
                featureInfo: {
                    address,
                    phone: "",
                    poiType: "",
                    name,
                    source: "0",
                    hotPointID: "store",
                    lonlat
                },
                featureId: "store"
            },
            popupId: "poi_store_0",
            position: null,
            offset: { x: 0, y: 50 }
        },
        viewport: {
            center: [longitude, latitude],
            zoom: 15,
            pitch: 0,
            bearing: 0,
            bound: "",
            mapNeedUpdate: true,
            location: false
        },
        mapInfo: {
            type: "pbf_vec",
            projection: "w",
            name: "cname",
            border: true,
            loaded: false
        }
    };
};

export const createTiandituNavigationUrl = async (
    destination: StoreNavigationDestination,
    request: TiandituShareRequest = fetch as TiandituShareRequest
): Promise<string> => {
    const body = new URLSearchParams({ json: JSON.stringify(buildTiandituShareState(destination)) }).toString();
    const response = await request(TIANDITU_SHARE_API, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8" },
        body
    });

    if (!response.ok) {
        throw new Error("天地图位置链接生成失败");
    }

    const result = await response.json();
    const data = typeof result.data === "string" ? JSON.parse(result.data) : result.data;
    if (Number(result.status) !== 200 || !data?.address) {
        throw new Error(result.message || "天地图位置链接生成失败");
    }

    return `${TIANDITU_SHARE_PAGE}/${encodeURIComponent(data.address)}`;
};

export const buildAmapNavigationUrl = (destination: StoreNavigationDestination): string => {
    const name = encodeURIComponent(destination.name || "目的地");
    return `https://ditu.amap.com/regeo?lng=${destination.longitude}&lat=${destination.latitude}&name=${name}&src=uriapi&innersrc=uriapi`;
};
