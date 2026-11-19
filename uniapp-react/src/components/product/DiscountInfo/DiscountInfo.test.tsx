import { cleanup, render } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import DiscountInfo from "./DiscountInfo";

vi.mock("@tarojs/components", () => ({ View: "div", Text: "span", Image: "img" }));
vi.mock("@nutui/nutui-react-taro", () => ({
    Popup: ({ children, visible }: { children?: ReactNode; visible?: boolean }) => (visible ? <div>{children}</div> : null)
}));
vi.mock("@/components/format-price/FormatPrice", () => ({
    default: ({ priceData }: { priceData?: string | number | null }) => <span>{String(priceData ?? "")}</span>
}));
vi.mock("@/utils", () => ({ staticResource: (path: string) => path }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));

afterEach(cleanup);

const baseTotal: any = {
    productAmount: "100.00",
    checkedCount: 1,
    discounts: "0.00",
    discountAfter: "100.00",
    discountCouponAmount: "0.00",
    discountDiscountAmount: "0.00"
};

describe("DiscountInfo serviceFee 门（Number>0 单独成门）", () => {
    it("serviceFee 为数字 0 时不渲染附加费用行，也不漏出文本「0」", () => {
        const { container } = render(<DiscountInfo show total={{ ...baseTotal, serviceFee: 0 }} />);
        expect(container.textContent).not.toContain("附加费用");
        // 漏渲染的「0」会成为 .discount-content 的裸文本子节点（价格文本都在 FormatPrice 的 span 内，不在此列）
        const bareTextNodes = Array.from(container.querySelector(".discount-content")?.childNodes ?? [])
            .filter((node) => node.nodeType === 3)
            .map((node) => node.textContent?.trim());
        expect(bareTextNodes).not.toContain("0");
    });

    it("serviceFee 为字符串「0.00」（线上 Decimal 序列化形态）时不渲染附加费用行", () => {
        const { container } = render(<DiscountInfo show total={{ ...baseTotal, serviceFee: "0.00" }} />);
        expect(container.textContent).not.toContain("附加费用");
    });

    it("serviceFee 缺失时不渲染附加费用行", () => {
        const { container } = render(<DiscountInfo show total={{ ...baseTotal }} />);
        expect(container.textContent).not.toContain("附加费用");
    });

    it("serviceFee 大于 0（字符串与数字）时照常渲染附加费用行", () => {
        const { container, rerender } = render(<DiscountInfo show total={{ ...baseTotal, serviceFee: "5.00" }} />);
        expect(container.textContent).toContain("附加费用");
        expect(container.textContent).toContain("5.00");
        rerender(<DiscountInfo show total={{ ...baseTotal, serviceFee: 5 }} />);
        expect(container.textContent).toContain("附加费用");
    });
});
