import { useUserStore } from "@/store/user";

/**
 * 分销员工具（自旧 utils/salesman.ts 等价移植）。
 * 旧实现的 salesman 逻辑此前分散在 product/productV1 页面内联
 * （saveSalesmanProducts）与 buy 组件（getSalemanId），此处按旧文件
 * 恢复统一入口：收口读取当前用户的分销员信息。
 */

export interface SalesmanInfo {
    salesmanId?: string | number;
    [key: string]: any;
}

/**
 * 读取当前登录用户的分销员信息。
 * 旧版读 useUserStore().userInfo.salesman；React 版 userInfo 结构一致，
 * 未登录/非分销员时返回 undefined。
 */
export function getCurrentSalesmanInfo(): SalesmanInfo | undefined {
    const userInfo: any = useUserStore.getState().userInfo || {};
    return (userInfo.salesman as SalesmanInfo) || undefined;
}

/** 当前分销员 ID（旧 getCurrentSalesmanId），无则 undefined。 */
export function getCurrentSalesmanId(): string | undefined {
    const info = getCurrentSalesmanInfo();
    if (info?.salesmanId === undefined || info?.salesmanId === null || info?.salesmanId === "") {
        return undefined;
    }
    return String(info.salesmanId);
}
