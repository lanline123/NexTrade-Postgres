/** 线下认款转账时间：精确到分钟，格式与日期选择器 value-format「YYYY-MM-DD HH:mm」一致 */
const pad = (value: number) => String(value).padStart(2, "0");

/** 秒级时间戳转为「YYYY-MM-DD HH:mm」，无效值返回空字符串 */
export const formatTransferTime = (timestamp?: number | string) => {
    if (!timestamp) return "";
    const date = new Date(Number(timestamp) * 1000);
    if (Number.isNaN(date.getTime())) return "";
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
};

/** 「YYYY-MM-DD HH:mm」转为秒级时间戳，未选择或无效时返回 0 */
export const parseTransferTime = (value?: string) => {
    if (!value) return 0;
    const time = new Date(`${value.replace(/-/g, "/")}:00`).getTime();
    return Number.isNaN(time) ? 0 : Math.floor(time / 1000);
};
