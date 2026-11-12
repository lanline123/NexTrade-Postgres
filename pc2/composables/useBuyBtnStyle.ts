/**
 * 统一解析 buyBtnStyle 配置
 * 兼容两种格式：
 * - 数字：buyBtnStyle: 1~8
 * - 对象：buyBtnStyle: { showBtn: boolean, btnType: number }
 */
export interface BuyBtnConfig {
    show: boolean;
    style: number;
}

export function normalizeBuyBtnStyle(raw: unknown): BuyBtnConfig {
    // 新格式：对象 { showBtn, btnType }
    if (raw && typeof raw === "object" && !Array.isArray(raw)) {
        const obj = raw as Record<string, unknown>;
        return {
            show: !!obj.showBtn,
            style: Number(obj.btnType) || 1
        };
    }
    if (typeof raw === "number" || typeof raw === "string") {
        const n = Number(raw);
        return { show: n > 0, style: n };
    }
    return { show: false, style: 0 };
}
