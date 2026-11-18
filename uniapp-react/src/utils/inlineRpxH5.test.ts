import { describe, expect, it } from "vitest";

import { convertInlineRpx } from "./inlineRpxH5";

describe("convertInlineRpx (H5 内联 rpx 收口)", () => {
    it("单值字号按 pxtransform 口径换算（÷40 rem）", () => {
        expect(convertInlineRpx("32rpx")).toBe("0.8rem");
        expect(convertInlineRpx("23rpx")).toBe("0.575rem");
        expect(convertInlineRpx("180rpx")).toBe("4.5rem");
    });

    it("复合值逐个换算、其余单位不动", () => {
        expect(convertInlineRpx("10rpx 20rpx")).toBe("0.25rem 0.5rem");
        expect(convertInlineRpx("calc(100% - 90rpx)")).toBe("calc(100% - 2.25rem)");
        expect(convertInlineRpx("2rpx solid #eee")).toBe("0.05rem solid #eee");
        expect(convertInlineRpx("-20rpx")).toBe("-0.5rem");
        expect(convertInlineRpx("1.5rpx")).toBe("0.0375rem");
    });

    it("不含 rpx 的值原样返回（不会二次换算）", () => {
        expect(convertInlineRpx("0.8rem")).toBe("0.8rem");
        expect(convertInlineRpx("16px")).toBe("16px");
        expect(convertInlineRpx("")).toBe("");
        expect(convertInlineRpx("bold")).toBe("bold");
    });
});
