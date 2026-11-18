import { describe, expect, it } from "vitest";

import { MENU_PARK_ICONS, SIGN_PARK_ICONS, parkIconSrc, resolveBannerIcon } from "./parkIcons";

describe("resolveBannerIcon (banner 搜索行菜单/签到取图与真值门控)", () => {
    it("编号 1-7/1-4 映射到 IconPark 字形（与管理端同一份表）", () => {
        expect(resolveBannerIcon(1, undefined, MENU_PARK_ICONS)).toEqual({ type: "park", name: "hamburger-button" });
        expect(resolveBannerIcon(7, undefined, MENU_PARK_ICONS)).toEqual({ type: "park", name: "more-app" });
        expect(resolveBannerIcon(2, "", MENU_PARK_ICONS)).toEqual({ type: "park", name: "application-menu" });
        expect(resolveBannerIcon(1, undefined, SIGN_PARK_ICONS)).toEqual({ type: "park", name: "calendar" });
        expect(resolveBannerIcon(4, undefined, SIGN_PARK_ICONS)).toEqual({ type: "park", name: "calendar-dot" });
    });

    it("字符串编号同样命中（存量数据口径）", () => {
        expect(resolveBannerIcon("3", undefined, MENU_PARK_ICONS)).toEqual({ type: "park", name: "list" });
    });

    it("0/空/未设置 = 不渲染（原版真值门控的隐藏态）", () => {
        expect(resolveBannerIcon(0, undefined, MENU_PARK_ICONS)).toBeNull();
        expect(resolveBannerIcon("0", undefined, MENU_PARK_ICONS)).toBeNull();
        expect(resolveBannerIcon(undefined, undefined, MENU_PARK_ICONS)).toBeNull();
        expect(resolveBannerIcon("", undefined, MENU_PARK_ICONS)).toBeNull();
    });

    it("custom 自定义图优先于编号字形；custom 无图则不渲染", () => {
        expect(resolveBannerIcon("custom", "https://example.com/a.png", MENU_PARK_ICONS)).toEqual({ type: "image", url: "https://example.com/a.png" });
        expect(resolveBannerIcon(1, "https://example.com/a.png", MENU_PARK_ICONS)).toEqual({ type: "image", url: "https://example.com/a.png" });
        expect(resolveBannerIcon("custom", "", MENU_PARK_ICONS)).toBeNull();
        expect(resolveBannerIcon("custom", undefined, MENU_PARK_ICONS)).toBeNull();
    });

    it("编号越界不渲染", () => {
        expect(resolveBannerIcon(8, undefined, MENU_PARK_ICONS)).toBeNull();
        expect(resolveBannerIcon(5, undefined, SIGN_PARK_ICONS)).toBeNull();
    });
});

describe("parkIconSrc", () => {
    it("生成内嵌 SVG data URI 且颜色已替换、无外部主机", () => {
        const src = parkIconSrc("earth");
        expect(src.startsWith("data:image/svg+xml,")).toBe(true);
        const svg = decodeURIComponent(src.slice("data:image/svg+xml,".length));
        expect(svg).toContain('viewBox="0 0 48 48"');
        expect(svg).toContain('stroke="#fff"');
        expect(svg).not.toContain("@C@");
        expect(svg).not.toContain("https://"); // 除 xmlns 命名空间外无任何外部主机引用
    });

    it("12 个字形全部有数据", () => {
        for (const name of [...MENU_PARK_ICONS, ...SIGN_PARK_ICONS, "earth"]) {
            const src = parkIconSrc(name);
            expect(decodeURIComponent(src)).toContain("<path");
        }
    });
});
