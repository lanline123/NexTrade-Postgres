import type { PcSearchModuleData } from "~/store/decorateRuntime";

export interface SearchDecorateModule {
    type?: string;
    url?: string;
    module?: Partial<PcSearchModuleData> | null;
}

export interface SearchDecorateResult {
    moduleList?: SearchDecorateModule[];
    data?: {
        moduleList?: SearchDecorateModule[];
    };
}

/** 读取有效搜索模块，避免历史错误目录的同名模块覆盖当前装修配置。 */
export function resolvePcSearchModuleData(result: SearchDecorateResult | null | undefined): Partial<PcSearchModuleData> | null {
    const moduleList = Array.isArray(result?.moduleList)
        ? result.moduleList
        : Array.isArray(result?.data?.moduleList) ? result.data.moduleList : [];
    const searchModule = moduleList.find((item) =>
        (item.type === "pcSearchResult" || item.type === "pc-search-result")
        && (!item.url || item.url === "pcSearch")
        && item.module
    );
    return searchModule?.module ?? null;
}
