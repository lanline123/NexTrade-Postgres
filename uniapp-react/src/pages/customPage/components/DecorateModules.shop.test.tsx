import { cleanup, render } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import DecorateModules from "./DecorateModules";

/**
 * 分发器级集成自检（2026-11-19 shopInfo 真实渲染空白修复）：
 * - 用假注册表替身记录每个模块实际收到的 props，断言店铺上下文数据流到达；
 * - 断言单模块抛错被 ModuleErrorBoundary 隔离、兄弟模块照常渲染。
 */
const receivedProps = vi.hoisted(() => ({ shopInfo: null as any, product: null as any }));

vi.mock("@tarojs/components", () => ({
    View: ({ children, ...props }: { children?: ReactNode }) => <div {...props}>{children}</div>,
    Image: () => null,
    Swiper: ({ children }: { children?: ReactNode }) => <div>{children}</div>,
    SwiperItem: ({ children }: { children?: ReactNode }) => <div>{children}</div>
}));
vi.mock("@/utils", () => ({ redirect: vi.fn(), getElementRect: vi.fn(async () => ({ height: 0 })) }));
vi.mock("@/utils/format", () => ({ imageFormat: (value: string) => value }));
vi.mock("@/pages/customPage/modules/registry", () => ({
    MODULE_REGISTRY: {
        navbar: () => <div data-testid="navbar" />,
        shopInfo: (props: any) => {
            receivedProps.shopInfo = props;
            return <div data-testid="shop-info">shop-info-ok</div>;
        },
        product: (props: any) => {
            receivedProps.product = props;
            return <div data-testid="product">product-ok</div>;
        },
        boom: () => {
            throw new Error("boom-module");
        }
    },
    STICKY_REGISTRY: {}
}));

/** 10079「默认模板」的真实 moduleList（MCP decorate_get_page 实取，字段略省尾部） */
const MODULE_LIST_10079 = [
    {
        isShow: true,
        label: "店铺信息",
        type: "shopInfo",
        moduleIndex: 1747970689080,
        module: { backgroundDefault: 1, customPic: { picThumb: "", picUrl: "" }, style: 1 }
    },
    {
        isShow: true,
        label: "商品",
        type: "product",
        moduleIndex: 1747970691231,
        module: { style: 2, products: { productCategoryId: [], productIds: [], productNumber: 0, productSelectType: 1, productTag: "" } }
    }
];

const SHOP_DETAIL_1667501180: any = {
    shopId: 1667501180,
    shopTitle: "恒芯再生·潍坊",
    shopLogo: "",
    productCount: 5,
    newProductCount: 2,
    collectShop: false
};

beforeEach(() => {
    receivedProps.shopInfo = null;
    receivedProps.product = null;
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(console, "warn").mockImplementation(() => {});
});
afterEach(cleanup);

describe("DecorateModules 店铺上下文分发 + 模块错误隔离", () => {
    it("shopInfo/shopId/onRefreshShopDetail 到达 shopInfo 模块，shopId 到达 product 模块", () => {
        const onRefresh = vi.fn();
        render(
            <DecorateModules
                modules={MODULE_LIST_10079}
                pageModles={{}}
                scrollTop={0}
                shopInfo={SHOP_DETAIL_1667501180}
                shopId={1667501180}
                onRefreshShopDetail={onRefresh}
            />
        );
        expect(receivedProps.shopInfo?.shopInfo?.shopTitle).toBe("恒芯再生·潍坊");
        expect(receivedProps.shopInfo?.shopId).toBe(1667501180);
        expect(receivedProps.shopInfo?.onRefreshShopDetail).toBe(onRefresh);
        expect(receivedProps.product?.shopId).toBe(1667501180);
    });

    it("不传店铺上下文时 shopInfo 模块不带 shopInfo（守卫空态口径），product 不带 shopId", () => {
        render(<DecorateModules modules={MODULE_LIST_10079} pageModles={{}} scrollTop={0} />);
        expect(receivedProps.shopInfo?.shopInfo).toBeUndefined();
        expect(receivedProps.product?.shopId).toBeUndefined();
    });

    it("单模块抛错被隔离：兄弟模块照常渲染，placeholder 模式出提示块且占位节点仍在", () => {
        const modules = [
            { isShow: true, type: "boom", moduleIndex: 1, module: {} },
            { isShow: true, type: "product", moduleIndex: 2, module: { style: 2 } }
        ];
        const { container, getByTestId, getByText } = render(
            <DecorateModules modules={modules} pageModles={{}} scrollTop={0} moduleErrorPlaceholder />
        );
        expect(getByTestId("product")).not.toBeNull();
        expect(getByText("该模块渲染失败，已跳过")).not.toBeNull();
        // 两个模块各占一个容器子节点（decoratePreview 布局槽位按 children 下标对齐不能位移）
        const root = container.firstElementChild as HTMLElement;
        expect(root.children.length).toBe(2);
    });
});
