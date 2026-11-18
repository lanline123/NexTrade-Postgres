import { afterEach, describe, expect, it, vi } from "vitest";

import navbarCss from "@/components/tig/tig-navbar/TigNavbar.scss?inline";
import tabbarCss from "@/components/tig/tig-tabbar/TigTabbar.scss?inline";
import { useFrameVars } from "@/utils/decorateFrame";
import { rpxToStyleLength } from "@/utils/styleLength";

vi.mock("@tarojs/taro", () => ({
    default: { pxTransform: (size) => `${size / 40}rem` }
}));

afterEach(() => {
    vi.unstubAllEnvs();
});

describe("member layout", () => {
    it.each([false, true])("keeps navigation spacers independent (reversed=%s)", (reversed) => {
        const style = document.createElement("style");
        style.textContent = (reversed ? [tabbarCss, navbarCss] : [navbarCss, tabbarCss]).join("\n");
        document.head.appendChild(style);
        try {
            const paddingRules = (className) => {
                const node = document.createElement("div");
                node.className = className;
                return Array.from(style.sheet.cssRules)
                    .filter((rule) => "selectorText" in rule && node.matches(rule.selectorText))
                    .flatMap((rule) => ["padding-top", "padding-bottom"].map((prop) => rule.style.getPropertyValue(prop)).filter(Boolean));
            };

            expect(paddingRules("tig-navbar__placeholder")).toEqual(["var(--nav-height)"]);
            expect(paddingRules("tig-tabbar__placeholder")).toEqual(["var(--tabbar-height)"]);
            expect(paddingRules("placeholder")).toEqual([]);
        } finally {
            style.remove();
        }
    });

    it.each([
        { platform: "h5", navbar: "3rem", padding: "0.8rem", margin: "0.4rem" },
        { platform: "weapp", navbar: "120rpx", padding: "32rpx", margin: "16rpx" }
    ])("writes supported inline lengths on $platform", ({ platform, navbar, padding, margin }) => {
        vi.stubEnv("TARO_ENV", platform);

        expect(rpxToStyleLength(120)).toBe(navbar);
        expect(useFrameVars({ frame: { innerPadding: 16, boxPaddingTop: 8 } })).toMatchObject({
            "--dec-inner-padding": padding,
            "--dec-box-padding-top": margin
        });
    });
});
