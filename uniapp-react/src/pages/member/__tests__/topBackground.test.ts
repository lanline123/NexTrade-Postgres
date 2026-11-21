import { describe, expect, it } from "vitest";

import { memberTopBackgroundCss, memberTopBackgroundFadeCss, resolveMemberTopBackground } from "../topBackground";

describe("member topBackground （会员中心顶部背景自定义，我方新增）", () => {
    it("未配置时恒为 null（存量页面零变化）", () => {
        expect(resolveMemberTopBackground(undefined)).toBeNull();
        expect(resolveMemberTopBackground(null)).toBeNull();
        expect(resolveMemberTopBackground({})).toBeNull();
        expect(resolveMemberTopBackground({ topBackground: null })).toBeNull();
        expect(resolveMemberTopBackground({ topBackground: { type: 0 } })).toBeNull();
        expect(resolveMemberTopBackground({ topBackground: { type: 9, color: "#000" } })).toBeNull();
    });

    it("纯色：生成同色渐变基础层，渐隐目标缺省 #f5f5f5", () => {
        const tb = resolveMemberTopBackground({ topBackground: { type: 1, color: "#1aad19" } });
        expect(tb).toEqual({ kind: "color", background: "linear-gradient(#1aad19, #1aad19)", fadeColor: "#f5f5f5" });
    });

    it("纯色：页面背景色已配置时渐隐目标跟随页面背景色", () => {
        const tb = resolveMemberTopBackground({ backgroundColor: "#eef2ff", topBackground: { type: 1, color: "#3355ff" } });
        expect(tb?.fadeColor).toBe("#eef2ff");
    });

    it("纯色：颜色为空视为未配置", () => {
        expect(resolveMemberTopBackground({ topBackground: { type: 1, color: "" } })).toBeNull();
        expect(resolveMemberTopBackground({ topBackground: { type: 1, color: "  " } })).toBeNull();
        expect(resolveMemberTopBackground({ topBackground: { type: 1 } })).toBeNull();
    });

    it("渐变：方向映射与 userInfo 口径一致（upDown/leftRight/diagonal）", () => {
        const tb = resolveMemberTopBackground({
            topBackground: { type: 2, gradient: { colorA: "#E0FCED", colorB: "#FDF1F3", gradientType: "upDown" } }
        });
        expect(tb?.kind).toBe("gradient");
        expect(tb?.background).toBe("linear-gradient(to bottom, #E0FCED, #FDF1F3)");
        const diag = resolveMemberTopBackground({
            topBackground: { type: 2, gradient: { colorA: "#aaa", colorB: "#bbb", gradientType: "diagonal" } }
        });
        expect(diag?.background).toBe("linear-gradient(135deg, #aaa, #bbb)");
    });

    it("渐变：缺字段时回落默认色与左右方向", () => {
        const tb = resolveMemberTopBackground({ topBackground: { type: 2 } });
        expect(tb?.background).toBe("linear-gradient(to right, #fcf6fc, #f3f3fe)");
    });

    it("背景图：PicSelect 对象与纯字符串两种存量形态都取 picUrl/原值", () => {
        const obj = resolveMemberTopBackground({ topBackground: { type: 3, image: { picUrl: "https://cdn.example.com/top.png", picThumb: "t" } } });
        expect(obj).toEqual({ kind: "image", image: "https://cdn.example.com/top.png", fadeColor: "#f5f5f5" });
        const str = resolveMemberTopBackground({ topBackground: { type: 3, image: "https://cdn.example.com/top2.png" } });
        expect(str?.image).toBe("https://cdn.example.com/top2.png");
        expect(resolveMemberTopBackground({ topBackground: { type: 3, image: { picUrl: "" } } })).toBeNull();
        expect(resolveMemberTopBackground({ topBackground: { type: 3 } })).toBeNull();
    });

    it("type 为字符串数字时同样生效（存量 JSON 规整前形态）", () => {
        const tb = resolveMemberTopBackground({ topBackground: { type: "1", color: "#123456" } });
        expect(tb?.kind).toBe("color");
    });

    it("CSS 组合：渐隐层在前、基础层在后", () => {
        const tb = resolveMemberTopBackground({ topBackground: { type: 1, color: "#000000" } });
        expect(tb).not.toBeNull();
        expect(memberTopBackgroundCss(tb!)).toBe(
            "linear-gradient(to bottom, transparent 55%, #f5f5f5 100%), linear-gradient(#000000, #000000)"
        );
        expect(memberTopBackgroundFadeCss(tb!)).toBe("linear-gradient(to bottom, transparent 55%, #f5f5f5 100%)");
    });
});
