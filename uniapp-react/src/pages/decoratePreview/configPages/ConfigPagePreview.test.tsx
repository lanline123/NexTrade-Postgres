import { cleanup, render } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import ConfigPagePreview from "./index";
import type { ConfigPageBranch } from "./resolve";

/**
 * 整页配置型预览分发（2026-11-19）：分发器按 branch.pageType 只渲染对应真实
 * 页面分支，并把编辑器实时 module（pt8 另带 type）原样透传——未保存改动靠这
 * 条透传进预览。三个分支组件以替身代之（各自的重依赖不在本测试范围）。
 */
vi.mock("@tarojs/components", () => ({
    View: ({ children, ...props }: { children?: ReactNode }) => <div {...props}>{children}</div>
}));

vi.mock("./LoginConfigPreview", () => ({
    default: ({ module }: { module: any }) => <div data-testid="login-preview">{JSON.stringify(module)}</div>
}));
vi.mock("./SearchConfigPreview", () => ({
    default: ({ module }: { module: any }) => <div data-testid="search-preview">{JSON.stringify(module)}</div>
}));
vi.mock("./ProductConfigPreview", () => ({
    default: ({ module, type }: { module: any; type: string }) => (
        <div data-testid="product-preview">
            {type}:{JSON.stringify(module)}
        </div>
    )
}));

afterEach(cleanup);

describe("ConfigPagePreview 整页分支分发", () => {
    it("pt5 渲染登录分支，module 原样透传", async () => {
        const branch: ConfigPageBranch = { pageType: 5, index: 0, type: "mobileLogin", module: { titleColor: "#333333" } };
        const { findByTestId, queryByTestId } = render(<ConfigPagePreview branch={branch} />);
        const el = await findByTestId("login-preview");
        expect(el.textContent).toContain("#333333");
        expect(queryByTestId("search-preview")).toBeNull();
        expect(queryByTestId("product-preview")).toBeNull();
    });

    it("pt6 渲染搜索分支，module 原样透传", async () => {
        const branch: ConfigPageBranch = { pageType: 6, index: 0, type: "mobileSearchResult", module: { listStyle: 2 } };
        const { findByTestId, queryByTestId } = render(<ConfigPagePreview branch={branch} />);
        const el = await findByTestId("search-preview");
        expect(el.textContent).toContain('"listStyle":2');
        expect(queryByTestId("login-preview")).toBeNull();
    });

    it("pt8 渲染商品详情分支，type 与 module 一并透传（A 版式判定靠 type）", async () => {
        const branch: ConfigPageBranch = { pageType: 8, index: 0, type: "productDetailA2", module: { buyBarStyle: 1 } };
        const { findByTestId, queryByTestId } = render(<ConfigPagePreview branch={branch} />);
        const el = await findByTestId("product-preview");
        expect(el.textContent).toContain("productDetailA2");
        expect(el.textContent).toContain('"buyBarStyle":1');
        expect(queryByTestId("login-preview")).toBeNull();
    });
});
