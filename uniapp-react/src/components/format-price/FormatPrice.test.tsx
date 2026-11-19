import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import FormatPrice from "./FormatPrice";

vi.mock("@tarojs/components", () => ({ View: "div" }));
vi.mock("@/utils", () => ({ isOverseas: () => false, isB2B: () => false }));
vi.mock("@/utils/i18n", () => ({ t: (s: string) => s }));
vi.mock("@/store/config", () => ({ useConfigStore: (select: Function) => select({ dollarSign: "￥" }) }));
vi.mock("@/store/currency", () => ({ useCurrencyStore: (select: Function) => select({ currentCurrencyData: {} }) }));
vi.mock("@/store/user", () => ({ useUserStore: (select: Function) => select({ userInfo: {} }) }));
afterEach(cleanup);

it("uses the smaller currency size for cents and honors an explicit decimal size", () => {
    const props = { fontStyle: { fontSize: "16px" }, currencyStyle: { fontSize: "12px", top: "-2px" } };
    const { getByText, rerender } = render(<FormatPrice {...props} priceData={100} />);
    expect(getByText(".00").style.fontSize).toBe("12px");
    expect(getByText(".00").style.top).toBe("");
    expect(getByText("100").style.fontSize).toBe("16px");
    rerender(<FormatPrice {...props} priceData="0.05" decimalsStyle={{ fontSize: "13px" }} />);
    expect(getByText("0")).toBeTruthy();
    expect(getByText(".05").style.fontSize).toBe("13px");
});
