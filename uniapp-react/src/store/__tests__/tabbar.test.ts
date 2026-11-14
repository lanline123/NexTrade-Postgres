import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/api/tabbar", () => ({
    getMobileNav: vi.fn()
}));

import { getMobileNav } from "@/api/tabbar";
import { useTabbarStore } from "../tabbar";
import { rpxToDevicePx } from "@/components/modules";

/**
 * 等价迁移自旧 store/__tests__/tabbarHeight.test.ts
 * （describe「底部导航高度与图标文字同步缩放」，旧版 Pinia+Vue）。
 * React 版状态层换成 Zustand：高度口径仍为 TABBAR_HEIGHT_RPX 经
 * rpxToDevicePx 按窗口宽（桩为 375）换算；导航数据拉取失败时
 * 回落默认四项（首页/分类/购物车/我的），成功时按新结构映射。
 */
describe("底部导航高度与数据回落（Zustand 版）", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        useTabbarStore.setState({ tabbarListDefault: [], currRoute: "" });
    });

    it("tabbarHeightNum 按 rpx→px 换算（375 宽）", () => {
        const state = useTabbarStore.getState();
        expect(state.tabbarHeightNum).toBeCloseTo(rpxToDevicePx(120, 375), 5);
        expect(state.tabbarHeightNum).toBeGreaterThan(0);
    });

    it("setCurrRoute 同步当前路由（供 pageMixin/tabbar 高亮）", () => {
        useTabbarStore.getState().setCurrRoute("/pages/cart/index");
        expect(useTabbarStore.getState().currRoute).toBe("/pages/cart/index");
    });

    it("导航接口失败时回落默认四项", async () => {
        (getMobileNav as any).mockRejectedValue(new Error("offline"));
        await useTabbarStore.getState().getTabbarList();
        const list = useTabbarStore.getState().tabbarListDefault;
        expect(list.length).toBe(4);
        expect(list.map((i: any) => i.link)).toEqual([
            "/pages/index/index",
            "/pages/productCate/index",
            "/pages/cart/index",
            "/pages/member/index"
        ]);
    });

    it("导航接口成功时按 picLink/picThumb/picTitle 映射", async () => {
        (getMobileNav as any).mockResolvedValue({
            navList: [{ picLink: "/pages/index/index", picThumb: "a.png", picActiveThumb: "b.png", picTitle: "首页" }],
            enableAnimation: 1
        });
        await useTabbarStore.getState().getTabbarList();
        const state = useTabbarStore.getState();
        expect(state.tabbarListDefault).toEqual([
            { link: "/pages/index/index", image: "a.png", activeImage: "b.png", text: "首页" }
        ]);
        expect(state.tabbarEnableAnimation).toBe(true);
    });
});
