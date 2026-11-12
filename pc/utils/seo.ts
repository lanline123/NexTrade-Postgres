export const stripSeoText = (value?: string | null) => {
    if (!value) {
        return "";
    }

    return value
        .replace(/<[^>]*>/g, " ")
        .replace(/&nbsp;/gi, " ")
        .replace(/&amp;/gi, "&")
        .replace(/\s+/g, " ")
        .trim();
};

export const truncateSeoText = (value?: string | null, maxLength: number = 160) => {
    const text = stripSeoText(value);
    if (!text) {
        return "";
    }

    return text.length > maxLength ? text.slice(0, maxLength) : text;
};

export const buildSeoKeywords = (items: Array<string | null | undefined>) => {
    const values = items
        .map((item) => stripSeoText(item))
        .filter((item) => item !== "");

    return [...new Set(values)].join(",");
};
