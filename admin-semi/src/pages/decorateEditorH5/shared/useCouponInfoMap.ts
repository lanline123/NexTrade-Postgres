/**
 * 优惠券 info 按 ID 拉取（对齐 view/admin 的 @/hooks/useCouponInfoMap 语义）
 *
 * 原版契约：模块数据里的优惠券条目只存 { couponId, btnText }，券面 info 不落库，
 * 由本 hook 按 couponId 从接口拉取后以 Map 形式供 Settings/Preview 消费。
 * 本实现以既有接口 /promotion/coupon/detail（后台优惠券管理同源）逐个拉取，
 * 模块级缓存跨组件共享（同一券只拉一次），removeCouponInfo 供删除券时清缓存。
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { apiGet } from "../../../api";

export interface CouponInfo {
    couponName?: string;
    couponType?: number;
    couponMoney?: number | string;
    couponDiscount?: number | string;
    minOrderAmount?: number | string;
    reduceType?: number;
    [key: string]: any;
}

const infoCache = new Map<number, CouponInfo>();
const inflight = new Map<number, Promise<void>>();

function normalizeCouponInfo(row: Record<string, any>): CouponInfo {
    return {
        couponName: row.couponName,
        couponType: row.couponType !== undefined && row.couponType !== null ? Number(row.couponType) : undefined,
        couponMoney: row.couponMoney,
        couponDiscount: row.couponDiscount,
        minOrderAmount: row.minOrderAmount,
        // 后端券记录自带 reduceType（1 满金额可用 / 2 无门槛）；缺失时按门槛金额推导
        reduceType:
            row.reduceType !== undefined && row.reduceType !== null
                ? Number(row.reduceType)
                : Number(row.minOrderAmount) > 0
                  ? 1
                  : 2
    };
}

export function useCouponInfoMap() {
    const [, setTick] = useState(0);
    const aliveRef = useRef(true);
    useEffect(() => {
        aliveRef.current = true;
        return () => {
            aliveRef.current = false;
        };
    }, []);

    const fetchCouponInfo = useCallback(async (ids: number[]) => {
        const wanted = Array.from(new Set((ids || []).map((id) => Number(id)).filter((id) => id > 0)));
        const jobs = wanted
            .filter((id) => !infoCache.has(id) && !inflight.has(id))
            .map((id) => {
                const job = (async () => {
                    try {
                        const detail = await apiGet<any>("/promotion/coupon/detail", { id });
                        const row = (detail?.record ?? detail) as Record<string, any> | undefined;
                        if (row && typeof row === "object" && (row.couponId !== undefined || row.couponName !== undefined)) {
                            infoCache.set(id, normalizeCouponInfo(row));
                        }
                    } catch {
                        // 拉取失败不写缓存，后续打开可重试；消费方按「信息缺失」口径展示
                    } finally {
                        inflight.delete(id);
                        if (aliveRef.current) setTick((t) => t + 1);
                    }
                })();
                inflight.set(id, job);
                return job;
            });
        await Promise.all(jobs);
    }, []);

    const removeCouponInfo = useCallback((id: number) => {
        if (infoCache.delete(Number(id))) {
            setTick((t) => t + 1);
        }
    }, []);

    return { couponInfoMap: infoCache, fetchCouponInfo, removeCouponInfo };
}
