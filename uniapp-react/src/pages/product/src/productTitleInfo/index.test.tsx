import { cleanup, render } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import ProductTitleInfo from "./index";

vi.mock("@tarojs/components", () => ({
    View: ({ children, ...props }: { children?: ReactNode }) => <div {...props}>{children}</div>,
    Text: ({ children, ...props }: { children?: ReactNode }) => <span {...props}>{children}</span>,
    Image: (props: any) => <img {...props} alt="" />
}));
vi.mock("@tarojs/taro", () => {
    const query: any = {
        select: () => query,
        boundingClientRect: () => query,
        exec: () => {}
    };
    return {
        default: {
            createSelectorQuery: () => query,
            showToast: vi.fn()
        }
    };
});
vi.mock("@/api/product/product", () => ({
    getCollectProduct: vi.fn(async () => false),
    delCollectProduct: vi.fn(async () => {}),
    updateCollectProduct: vi.fn(async () => {})
}));
vi.mock("@/components/format-price/FormatPrice", () => ({
    default: ({ priceData }: { priceData?: string | number | null }) => <span>{String(priceData ?? "")}</span>
}));
vi.mock("@/store/config", () => ({
    useConfigStore: (select: Function) => select({ showSelledCount: 1, showMarketprice: 1 })
}));
vi.mock("@/utils", () => ({
    staticResource: (path: string) => path,
    rpx2px: (value: number) => value / 2
}));
vi.mock("@/utils/b2bPrice", () => ({ isShowB2bPriceText: () => false }));
vi.mock("@/utils/shareLogin", () => ({ ensureShareLogin: () => true }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));
vi.mock("../productShare", () => ({ default: () => null }));
vi.mock("../productGrouponTitle", () => ({ default: () => null }));
vi.mock("../productSeckillTitle", () => ({ default: () => null }));
vi.mock("../productDiscountTitle", () => ({ default: () => null }));
vi.mock("../productPresaleTitle", () => ({ default: () => null }));

afterEach(cleanup);

const productInfo: any = { productId: 1, productName: "测试商品", productBrief: "", marketPrice: "0" };

describe("ProductTitleInfo 促销行可见性（无可见标签不占位，与瀑布流卡 isActivityVisible 同口径）", () => {
    it("只有不可见条目（type=0）时不渲染活动标签行与领券按钮", () => {
        const { container } = render(
            <ProductTitleInfo productInfo={productInfo} productPrice="10" promotionList={[{ promotionId: 11, type: 0, promotionName: "", data: {} } as any]} />
        );
        expect(container.querySelector(".activity-box")).toBeNull();
        expect(container.textContent).not.toContain("领券");
    });

    it("不可见条目在前、可见条目在后时只渲染可见标签", () => {
        const { container } = render(
            <ProductTitleInfo
                productInfo={productInfo}
                productPrice="10"
                promotionList={
                    [
                        { promotionId: 11, type: 0, promotionName: "", data: {} },
                        { promotionId: 12, type: 3, promotionName: "满减活动", data: {} }
                    ] as any
                }
            />
        );
        expect(container.querySelector(".activity-box")).not.toBeNull();
        expect(container.querySelectorAll(".promotion")).toHaveLength(1);
        expect(container.textContent).toContain("满减");
        expect(container.textContent).toContain("领券");
    });

    it("type=2 无券面描述不可见、带描述可见", () => {
        const { container, rerender } = render(
            <ProductTitleInfo productInfo={productInfo} productPrice="10" promotionList={[{ promotionId: 13, type: 2, promotionName: "券", data: {} } as any]} />
        );
        expect(container.querySelector(".activity-box")).toBeNull();
        rerender(
            <ProductTitleInfo
                productInfo={productInfo}
                productPrice="10"
                promotionList={[{ promotionId: 13, type: 2, promotionName: "券", data: { promotionDesc: "满10减5" } } as any]}
            />
        );
        expect(container.querySelector(".activity-box")).not.toBeNull();
        expect(container.textContent).toContain("满10减5");
    });

    it("A1 优惠行：只有不可见条目时不渲染；文案取第一个可见条目的名称", () => {
        const { container, rerender } = render(
            <ProductTitleInfo
                layoutStyle="A1"
                productInfo={productInfo}
                productPrice="10"
                promotionList={[{ promotionId: 11, type: 0, promotionName: "不应显示", data: {} } as any]}
            />
        );
        expect(container.querySelector(".vortmall-coupon")).toBeNull();
        expect(container.textContent).not.toContain("不应显示");
        rerender(
            <ProductTitleInfo
                layoutStyle="A1"
                productInfo={productInfo}
                productPrice="10"
                promotionList={
                    [
                        { promotionId: 11, type: 0, promotionName: "不应显示", data: {} },
                        { promotionId: 12, type: 3, promotionName: "满减活动", data: {} }
                    ] as any
                }
            />
        );
        expect(container.querySelector(".vortmall-coupon")).not.toBeNull();
        expect(container.querySelector(".vortmall-coupon-text")?.textContent).toBe("满减活动");
    });
});
