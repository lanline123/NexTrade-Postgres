import { cleanup, render } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import ShopInfoModule from "./index";

vi.mock("@tarojs/components", () => ({
    // Taro 的 style 可为字符串，react-dom 不接受：mock 里丢弃 style 只验结构与文案
    View: ({ children, style, ...props }: { children?: ReactNode; style?: unknown }) => <div {...props}>{children}</div>,
    Input: (props: any) => <input {...props} />
}));
vi.mock("@tarojs/taro", () => ({
    default: { showToast: vi.fn(), showModal: vi.fn(), navigateTo: vi.fn() }
}));
vi.mock("@/components/tig/tig-image/TigImage", () => ({
    default: ({ src }: { src?: string }) => <img data-testid="tig-image" src={src} alt="" />
}));
vi.mock("@/api/shop/shop", () => ({ shopCollection: vi.fn() }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));
vi.mock("@/utils/format", () => ({ imageFormat: (value: string) => value }));

afterEach(cleanup);

/** 10079「默认模板」里 shopInfo 模块的真实 module 数据形状（MCP decorate_get_page 实取） */
const MODULE_10079 = {
    backgroundDefault: 1,
    customPic: { picThumb: "", picUrl: "" },
    frame: {
        backgroundColor: "#ffffff",
        boxPadding: 10,
        boxPaddingBottom: 5,
        boxPaddingTop: 5,
        boxRadius: 0,
        innerPadding: 0,
        itemBackgroundColor: "",
        itemHeight: 35,
        itemRadius: 3,
        textColor: ""
    },
    style: 1
};

describe("ShopInfoModule 守卫（2026-11-19：通用分发器不传 shopInfo 时整页空白回归）", () => {
    it("shopInfo 完全缺失时不抛错，渲染结构空态（未收藏 / 计数 0）", () => {
        const { container } = render(<ShopInfoModule module={MODULE_10079} shopInfo={undefined} />);
        expect(container.querySelector(".module-store_info")).not.toBeNull();
        expect(container.querySelector(".cap-store-banner")).not.toBeNull();
        expect(container.textContent).toContain("收藏");
        expect(container.textContent).not.toContain("已收藏");
        expect(container.textContent).toContain("全部商品 0");
        expect(container.textContent).toContain("新品 0");
    });

    it("传入真实店铺详情时渲染店名与计数（shopId=1667501180 恒芯再生·潍坊形状）", () => {
        const shopInfo: any = {
            shopId: 1667501180,
            shopTitle: "恒芯再生·潍坊",
            shopLogo: "",
            productCount: 5,
            newProductCount: 2,
            collectShop: false
        };
        const { container } = render(<ShopInfoModule module={MODULE_10079} shopInfo={shopInfo} />);
        expect(container.textContent).toContain("恒芯再生·潍坊");
        expect(container.textContent).toContain("全部商品 5");
        expect(container.textContent).toContain("新品 2");
    });
});
