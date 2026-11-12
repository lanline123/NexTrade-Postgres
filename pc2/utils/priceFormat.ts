export interface PriceParts {
    integer: string;
    decimals?: string;
}

/** 将价格拆分为整数和小数展示部分；默认统一保留两位小数。 */
export const splitPriceParts = (value: string | number | null | undefined, completeDecimal = true): PriceParts => {
    const num = typeof value === "number" ? String(value) : value;
    if (!num) return { integer: "0", decimals: ".00" };

    const match = num.match(/^(\d+)(\.\d+)?$/);
    if (!match) return { integer: "0", decimals: ".00" };

    if (completeDecimal) {
        const [integer, decimals] = Number(num).toFixed(2).split(".");
        return { integer, decimals: `.${decimals}` };
    }

    return { integer: match[1] || "0", decimals: match[2] };
};
