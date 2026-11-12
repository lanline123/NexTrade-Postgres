/** 展示层格式化。金额按人民币原值展示，不做换币换算（支付仍走原值）。 */

/** 金额格式化。入参放宽到 unknown，便于装修模块等动态数据结构直接传入。 */
export function formatPrice(value: unknown): string {
    const raw = String(value ?? '').replace(/[^\d.-]/g, '');
    const amount = Number(raw);
    return Number.isFinite(amount) ? `¥${amount.toFixed(2)}` : '¥0.00';
}

/** 秒级时间戳或后端格式化好的字符串 → 展示用日期。 */
export function formatDateTime(value: number | string | null | undefined): string {
    if (value === null || value === undefined || value === '') return '';
    if (typeof value === 'string' && value.includes('-')) return value;
    const seconds = Number(value);
    if (!Number.isFinite(seconds) || seconds <= 0) return '';
    const date = new Date(seconds * 1000);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

/** SKU 可能是 JSON 串、对象数组或字符串数组，统一成「颜色:红 尺码:M」。 */
export function skuText(value: unknown): string {
    if (!value) return '';
    let parsed: unknown = value;
    if (typeof value === 'string') {
        try {
            parsed = JSON.parse(value);
        } catch {
            return value;
        }
    }
    if (Array.isArray(parsed)) {
        return parsed
            .map((entry) => {
                if (typeof entry === 'string') return entry;
                if (entry && typeof entry === 'object') {
                    const item = entry as { name?: string; value?: string; attrName?: string; attrValue?: string };
                    const name = item.name ?? item.attrName ?? '';
                    const val = item.value ?? item.attrValue ?? '';
                    return name && val ? `${name}:${val}` : val || name;
                }
                return '';
            })
            .filter(Boolean)
            .join(' ');
    }
    return typeof parsed === 'string' ? parsed : '';
}

export function productImage(item: { picThumb?: string; productPicThumb?: string; productImage?: string }): string {
    return item.picThumb || item.productPicThumb || item.productImage || '/assets/avatar/default.gif';
}
