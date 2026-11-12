/**
 * PC 首页新旧版本分流判定。
 *
 * 主判据：后端透传的 pageModule.homeStyleVersion（Admin 持久化，预览 + 正式两路都返回）。
 * 兜底：无标记的老数据按模块类型推断——新套必为 kebab（含 "-"），旧套全 camelCase。
 */
export function isNewHome(pageModule?: Record<string, any> | null, moduleList: { type?: string }[] = []): boolean {
    const v = pageModule?.homeStyleVersion;
    if (v === "new") return true;
    if (v === "old") return false;
    return moduleList.some((m) => String(m?.type ?? "").includes("-"));
}
