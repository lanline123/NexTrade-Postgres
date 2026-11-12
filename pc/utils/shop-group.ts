// 兼容历史单选配置；无有效分组时不生成全量列表链接。
export function groupLinkUrl(link: { path?: string; groupIds?: unknown; id?: unknown; data?: Record<string, any> }): string {
    const raw = link.groupIds ?? link.data?.groupIds ?? [link.id ?? link.data?.id];
    const values = Array.isArray(raw) ? raw : String(raw).split(",");
    const ids = [...new Set(values.map(Number).filter((id) => Number.isSafeInteger(id) && id > 0 && id <= 2147483647))];
    if (!ids.length) return "";
    return (link.path === "storeGroup" ? "/store/list" : "/shop/list") + "?groupIds=" + ids.join(",");
}

// 保留显式空值及非法值，由后端校验，避免错误链接退化为全量列表。
export function groupListParams(query: Record<string, unknown>, shopType: number): { groupIds?: string; shopType?: number } {
    if (query.groupIds === undefined) return {};
    return { groupIds: Array.isArray(query.groupIds) ? query.groupIds.join(",") : String(query.groupIds ?? ""), shopType };
}
