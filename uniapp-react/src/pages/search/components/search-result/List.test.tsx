import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import SearchProductList from "./List";

vi.mock("@tarojs/components", () => ({ View: "div", Text: "span" }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));
vi.mock("@/utils", () => ({ redirect: vi.fn() }));
vi.mock("@/utils/inquiry", () => ({
    resolveInquiryState: () => "none",
    withInquiryParam: (url: string) => url
}));
vi.mock("@/utils/productSoldOut", () => ({ shouldShowSoldOut: () => false }));
vi.mock("@/components/tig/tig-lazy-image/TigLazyImage", () => ({ default: () => null }));
vi.mock("@/components/inquiry-price/InquiryPrice", () => ({
    default: ({ priceData }: { priceData?: string | number }) => <span>{String(priceData ?? "")}</span>
}));
vi.mock("@/components/product/specification", () => ({ default: () => null }));

afterEach(cleanup);

const product: any = { productId: 1, productName: "测试商品", picUrl: "", virtualSales: 3, price: "10.00", shop: { shopTitle: "自营" } };
const renderWithActivity = (activityInfo: any[]) =>
    render(<SearchProductList productList={[product]} promotionList={{ 1: { activityInfo } }} config={{} as any} />);

describe("SearchProductList 活动行可见性收口（拼团标保留）", () => {
    it("只有不可见条目（type=0）时不渲染活动行", () => {
        const { container } = renderWithActivity([{ promotionId: 11, type: 0, data: {} }]);
        expect(container.querySelector(".activity-box")).toBeNull();
        expect(container.querySelector(".promotion")).toBeNull();
    });

    it("type=7 拼团条目走独立「拼团」标，不被可见性过滤吞掉", () => {
        const { container } = renderWithActivity([{ promotionId: 12, type: 7, data: {} }]);
        expect(container.querySelector(".activity-box")).not.toBeNull();
        expect(container.querySelector(".groupon-tag")?.textContent).toBe("拼团");
        expect(container.querySelector(".promotion")).toBeNull();
    });

    it("不可见条目在前、可见条目在后时只渲染可见标签", () => {
        const { container } = renderWithActivity([
            { promotionId: 11, type: 0, data: {} },
            { promotionId: 13, type: 3, data: {} }
        ]);
        expect(container.querySelector(".activity-box")).not.toBeNull();
        expect(container.querySelectorAll(".promotion")).toHaveLength(1);
        expect(container.textContent).toContain("满减");
    });

    it("type=2 优惠券按原口径不展示（即使带券面描述）", () => {
        const { container } = renderWithActivity([{ promotionId: 14, type: 2, data: { promotionDesc: "满10减5" } }]);
        expect(container.querySelector(".activity-box")).toBeNull();
        expect(container.textContent).not.toContain("满10减5");
    });

    it("type=8 抽奖条目不可见，不占行", () => {
        const { container } = renderWithActivity([{ promotionId: 15, type: 8, data: {} }]);
        expect(container.querySelector(".activity-box")).toBeNull();
    });
});
