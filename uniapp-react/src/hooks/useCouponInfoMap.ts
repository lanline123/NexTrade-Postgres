import { useCallback, useEffect, useRef, useState } from "react";

import { getCouponListByIds } from "@/api/coupon/coupon";
import type { CouponFilterResult } from "@/types/coupon/coupon.d";

/**
 * 优惠券信息 Map（自旧 hooks/useCouponInfoMap.ts 等价移植，恢复可复用 hook）。
 * 此前同逻辑以模块级单例内联在 newcomerCombo 装修模块中（全仓唯一消费方）；
 * 本 hook 与之共用同一份模块级单例缓存与在途去重，行为一致：
 * - couponInfoMap：模块级单例 Map<couponId, CouponFilterResult>
 * - fetchCouponInfo(ids)：跳过已缓存 id，按 ids key 对并发请求去重合并
 * - removeCouponInfo(couponId)：删除单条缓存
 * 返回的 version 在缓存变化时递增，消费组件可用它触发重渲染
 * （React 版替代旧 Vue Ref<Map> 的响应式触发）。
 */
const couponInfoMap = new Map<number, CouponFilterResult>();
const inFlight = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();
let version = 0;

function notify() {
    version += 1;
    listeners.forEach((fn) => fn());
}

async function fetchCouponInfoShared(ids: number[]): Promise<void> {
    const missing = Array.from(new Set(ids)).filter((id) => id && !couponInfoMap.has(id));
    if (!missing.length) return;
    const key = missing.join(",");
    const existing = inFlight.get(key);
    if (existing) return existing;
    const task = (async () => {
        try {
            const res: any = await getCouponListByIds(missing);
            const list: CouponFilterResult[] = res?.records || res?.data || res || [];
            (Array.isArray(list) ? list : []).forEach((item: any) => {
                const id = Number(item?.couponId ?? item?.id);
                if (id) couponInfoMap.set(id, item);
            });
            notify();
        } catch {
            // 静默降级：与旧实现/newcomerCombo 内联版一致，拉取失败不阻塞渲染
        } finally {
            inFlight.delete(key);
        }
    })();
    inFlight.set(key, task);
    return task;
}

export function useCouponInfoMap() {
    const [, setVersion] = useState(version);
    const mounted = useRef(true);

    useEffect(() => {
        mounted.current = true;
        const listener = () => {
            if (mounted.current) setVersion(version);
        };
        listeners.add(listener);
        return () => {
            mounted.current = false;
            listeners.delete(listener);
        };
    }, []);

    const fetchCouponInfo = useCallback((ids: number[]) => fetchCouponInfoShared(ids), []);

    const removeCouponInfo = useCallback((couponId: number) => {
        if (couponInfoMap.delete(couponId)) notify();
    }, []);

    return { couponInfoMap, fetchCouponInfo, removeCouponInfo };
}

export default useCouponInfoMap;
