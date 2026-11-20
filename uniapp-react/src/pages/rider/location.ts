import Taro from "@tarojs/taro";

/**
 * 骑手定位与导航唤起（我方新增）：
 * - 定位仅在状态动作（接单/取货/送达）时取一次（动作点打点口径），
 *   授权失败/超时静默降级为空，绝不阻塞业务动作（H5 无后台定位，MVP 口径）；
 * - 导航：有坐标时 H5 跳高德 URI Scheme 网页导航，其余端走 Taro.openLocation；
 *   无坐标时复制地址兜底。
 */
export async function currentCoords(): Promise<{ longitude?: number; latitude?: number }> {
    try {
        const res = await Taro.getLocation({ type: "gcj02" });
        if (typeof res.longitude === "number" && typeof res.latitude === "number") {
            return { longitude: res.longitude, latitude: res.latitude };
        }
    } catch {
        // 定位不可用：静默降级
    }
    return {};
}

export function navigateToPlace(name: string, address: string, lng?: number | null, lat?: number | null) {
    if (typeof lng === "number" && typeof lat === "number" && lng !== 0 && lat !== 0) {
        if (process.env.TARO_ENV === "h5") {
            const url = `https://uri.amap.com/navigation?to=${lng},${lat},${encodeURIComponent(name || address)}&mode=car&policy=1&src=nextrade&callnative=1`;
            window.location.href = url;
            return;
        }
        Taro.openLocation({ longitude: lng, latitude: lat, name: name || address, address }).catch(() => {
            Taro.showToast({ title: "无法打开地图", icon: "none" });
        });
        return;
    }
    Taro.setClipboardData({ data: address }).then(() => {
        Taro.showToast({ title: "地址已复制，请打开地图搜索", icon: "none" });
    });
}

export function callPhone(phone?: string) {
    if (!phone) {
        Taro.showToast({ title: "暂无电话", icon: "none" });
        return;
    }
    Taro.makePhoneCall({ phoneNumber: phone }).catch(() => undefined);
}

export function fmtTime(ts?: number | null): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    const p = (n: number) => String(n).padStart(2, "0");
    return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

export const DELIVERY_STATUS_TEXT: Record<number, string> = {
    10: "待派发",
    20: "待取货",
    30: "配送中",
    40: "已完成",
    50: "已取消"
};
