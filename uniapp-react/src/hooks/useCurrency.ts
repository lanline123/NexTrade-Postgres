import { useCurrencyStore } from "@/store/currency";
import { isOverseas } from "@/utils";

/**
 * 货币读取 hook（自旧 hooks/useCurrency.ts 等价移植，恢复统一入口）。
 * 功能本体已由 Zustand store 承接（useCurrencyStore 直读 storage、
 * useConfigStore 提供站点配置）；本 hook 按旧签名聚合四个值，
 * 供新代码以旧方式消费。React 版返回普通值（随 store 订阅更新），
 * 对应旧版返回的四个 Ref 解包后的值：
 * - currency：当前货币代码/标识（旧 getCurrency：海外版取当前货币名，
 *   国内版取基础货币名；此处以 CurrencyItem.name 为准）
 * - currencyName：当前货币名称
 * - baseCurrency / baseCurrencyName：默认（isDefault）货币的代码与名称
 */
export const useCurrency = () => {
    const currentCurrencyData = useCurrencyStore((s) => s.currentCurrencyData);
    const currencyList = useCurrencyStore((s) => s.currencyList);
    // 注：旧版还读了 config store 的基础货币字段；React 版 config store
    // 无货币字段，基础货币统一以 currencyList 的 isDefault 项为准（与 store 逻辑一致）

    const base = currencyList.find((item) => item.isDefault === 1) || currentCurrencyData;
    const currencyName = currentCurrencyData?.name || "";
    const currency = isOverseas() ? currencyName : base?.name || currencyName;

    return {
        currency,
        currencyName,
        baseCurrency: base?.name || "",
        baseCurrencyName: base?.name || ""
    };
};

export default useCurrency;
