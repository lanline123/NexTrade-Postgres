import { apiGet } from '@/lib/client-api';

/**
 * 秒杀 / 拼团 数据层。
 * Vue 基准：api/home/home.ts（getHomeSeckill、getHomeGrouponList）、api/joining/joining.ts（getProductList）。
 *
 * **真实接口实测（2026-11-13）**：
 * - GET home/home/getSeckill?page=&size= → records[{ seckillId, seckillPrice, seckillStock, seckillSales,
 *   seckillStartTime(秒级时间戳), seckillEndTime, remainSeconds, started, soldPercent, skuId, ...product }]
 * - GET promotion/groupon/itemList?page=&size= → records[{ grouponPromotionItemId, minGrouponPrice,
 *   minGroupNum, productPrice, groupNum(JSON 字符串), joinNumCount, canJoin, restrictTip, startTime, endTime, ...product }]
 *   注意：拼团**没有** `home/home/getGroupon`（404），首页拼团模块也走 itemList。
 * - GET promotion/groupon/record/list → 拼团记录（当前库为空）
 */
export interface PromotionProductBase {
    productId?: number;
    productName?: string;
    productSn?: string;
    picThumb?: string;
    picUrl?: string;
    productBrief?: string;
    productPrice?: string | number;
    marketPrice?: string | number;
}

export interface SeckillItem extends PromotionProductBase {
    seckillId?: number;
    skuId?: number;
    seckillPrice?: string | number;
    seckillStock?: number;
    seckillSales?: number;
    seckillStartTime?: number;
    seckillEndTime?: number;
    remainSeconds?: number;
    started?: boolean;
    soldPercent?: number;
    canJoin?: boolean;
    restrictTip?: string;
}

export interface GrouponItem extends PromotionProductBase {
    grouponPromotionItemId?: number;
    minGrouponPrice?: string | number;
    minGroupNum?: number;
    joinNumCount?: number;
    canJoin?: boolean;
    restrictTip?: string;
    /** JSON 字符串：[{ groupNumKey, groupNum }] */
    groupNum?: string;
    startTime?: string;
    endTime?: string;
}

export interface PromotionPage<T> {
    records?: T[];
    total?: number;
}

export async function fetchSeckillList(page = 1, size = 20): Promise<PromotionPage<SeckillItem>> {
    const result = await apiGet<PromotionPage<SeckillItem>>('home/home/getSeckill', { page, size });
    return { records: result?.records ?? [], total: Number(result?.total ?? 0) };
}

export async function fetchGrouponList(page = 1, size = 20): Promise<PromotionPage<GrouponItem>> {
    const result = await apiGet<PromotionPage<GrouponItem>>('promotion/groupon/itemList', { page, size });
    return { records: result?.records ?? [], total: Number(result?.total ?? 0) };
}

/** 团人数取 groupNum 里最大的那档（Vue 的 getMaxGroupNum 同口径）。 */
export function maxGroupNum(raw?: string): number {
    if (!raw) return 0;
    try {
        const list = JSON.parse(raw) as { groupNum?: number }[];
        if (!Array.isArray(list) || list.length === 0) return 0;
        return Math.max(...list.map((item) => Number(item.groupNum) || 0));
    } catch {
        return 0;
    }
}

/** 秒杀已售比例：优先用后端 soldPercent，缺失时按 (已售 / (库存+已售)) 现算。 */
export function soldPercentOf(item: SeckillItem): number {
    if (typeof item.soldPercent === 'number' && item.soldPercent > 0) return Math.min(100, item.soldPercent);
    const sold = Number(item.seckillSales ?? 0);
    const stock = Number(item.seckillStock ?? 0);
    const base = stock + sold;
    return base > 0 ? Math.min(100, Math.round((sold / base) * 100)) : 0;
}

export interface Countdown {
    days: number;
    hours: number;
    minutes: number;
    seconds: number;
    /** 剩余毫秒，<=0 表示已到点 */
    remain: number;
    text: string;
}

/** 把毫秒差拆成天/时/分/秒，并给一份「01天 02:03:04」文本。 */
export function splitCountdown(remainMs: number): Countdown {
    const remain = Math.max(0, remainMs);
    const totalSeconds = Math.floor(remain / 1000);
    const days = Math.floor(totalSeconds / 86400);
    const hours = Math.floor((totalSeconds % 86400) / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    const pad = (value: number) => String(value).padStart(2, '0');
    return {
        days,
        hours,
        minutes,
        seconds,
        remain,
        text: `${pad(days)}天 ${pad(hours)}:${pad(minutes)}:${pad(seconds)}`,
    };
}

/** `endTime` 可能是 "2026-11-10 23:19:55" 或秒级时间戳字符串。 */
export function toTimestamp(value?: string | number | null): number {
    if (value === undefined || value === null || value === '') return 0;
    if (typeof value === 'number') return value > 1e12 ? value : value * 1000;
    const numeric = Number(value);
    if (Number.isFinite(numeric) && numeric > 0) return numeric > 1e12 ? numeric : numeric * 1000;
    // Safari 不认 "YYYY-MM-DD HH:mm:ss"，换成 ISO 形式
    const parsed = Date.parse(String(value).replace(' ', 'T'));
    return Number.isFinite(parsed) ? parsed : 0;
}
