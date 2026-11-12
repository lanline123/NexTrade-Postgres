/**
 * 计算折扣百分比（便宜了百分之多少），始终向下取整。
 *
 * 取整前先 toFixed(8)：消除 IEEE 754 浮点噪声（通常出现在 1e-15 量级），
 * 同时保留真实的小数位，避免把正常折扣抹成 0。
 *
 * @param currentPrice  - 现价（促销价）
 * @param originalPrice - 原价（划线价）
 * @param decimals      - 保留的小数位（0=整数，1=一位小数）
 * @returns 折扣百分比，如 5 或 5.2；入参非法时返回 0
 */
export function calcDiscountPercent(currentPrice: number, originalPrice: number, decimals: number = 1): number {
    if (!Number.isFinite(originalPrice) || !Number.isFinite(currentPrice) || originalPrice <= 0 || currentPrice >= originalPrice) {
        return 0;
    }
    const percent = ((originalPrice - currentPrice) / originalPrice) * 100;
    const factor = Math.pow(10, decimals);
    return Math.floor(Number((percent * factor).toFixed(8))) / factor;
}
