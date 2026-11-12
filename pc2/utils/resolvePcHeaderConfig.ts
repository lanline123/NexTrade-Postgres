import type { PcHeaderConfig } from "~/types/decorate/pcHeader";

function isPlainObject(val: unknown): val is Record<string, unknown> {
    return !!val && typeof val === "object" && !Array.isArray(val);
}

/**
 * 从后端返回中解析出 PC 头部装修配置。
 * 兼容多种字段命名/包裹层级（pcHeader / paHeader / pageModule 等）。
 * 后端 home/home/pcHeader 返回形如 { pcHeader: {...} }，命中 root.pcHeader。
 */
export function resolvePcHeaderConfigFromResult(result: unknown): PcHeaderConfig | null {
    const root = isPlainObject(result) ? result : null;
    if (!root) return null;

    const data = isPlainObject(root.data) ? root.data : null;
    const pageModule = isPlainObject(root.pageModule) ? root.pageModule : null;
    const dataPageModule = data && isPlainObject(data.pageModule) ? data.pageModule : null;

    const candidates: unknown[] = [
        root.paHeader,
        data?.paHeader,
        pageModule?.paHeader,
        dataPageModule?.paHeader,
        root.pcHeader,
        data?.pcHeader,
        pageModule?.pcHeader,
        dataPageModule?.pcHeader
    ];

    for (const raw of candidates) {
        if (!isPlainObject(raw)) continue;
        if ("styleType" in raw) return raw as unknown as PcHeaderConfig;
        const maybePcHeader = raw.pcHeader;
        if (isPlainObject(maybePcHeader) && "styleType" in maybePcHeader) return maybePcHeader as unknown as PcHeaderConfig;
        const maybeConfig = raw.config;
        if (isPlainObject(maybeConfig) && "styleType" in maybeConfig) return maybeConfig as unknown as PcHeaderConfig;
        return raw as unknown as PcHeaderConfig;
    }
    return null;
}
