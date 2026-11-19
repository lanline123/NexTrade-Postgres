import { render } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import ActivityCard, { isActivityVisible } from "./ActivityCard";

vi.mock("@tarojs/components", () => ({ Text: "span", View: "div" }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));

it("renders no digits for a zero activity type and keeps a valid zero coupon description", () => {
    const { container, rerender } = render(<ActivityCard activityInfo={{ type: 0 }} />);
    expect(container.textContent).toBe("");
    rerender(<ActivityCard activityInfo={{ type: 2, data: { promotionDesc: "0元券" } }} />);
    expect(container.textContent).toBe("券0元券");
});

it("isActivityVisible matches exactly what the card renders as a visible label", () => {
    // 无文案条目：0（占位）、7（拼团，卡面另有内联标）、8（抽奖）、未知 type、空对象
    expect(isActivityVisible({ type: 0 })).toBe(false);
    expect(isActivityVisible({ type: 7 })).toBe(false);
    expect(isActivityVisible({ type: 8 })).toBe(false);
    expect(isActivityVisible({ type: 99 })).toBe(false);
    expect(isActivityVisible({})).toBe(false);
    expect(isActivityVisible(undefined)).toBe(false);
    expect(isActivityVisible(null)).toBe(false);
    // 有文案的活动类型
    expect(isActivityVisible({ type: 1 })).toBe(true);
    expect(isActivityVisible({ type: 3 })).toBe(true);
    expect(isActivityVisible({ type: 6 })).toBe(true);
    // 优惠券：带券面描述才可见（含合法的「0元券」文案）
    expect(isActivityVisible({ type: 2, data: { promotionDesc: "0元券" } })).toBe(true);
    expect(isActivityVisible({ type: 2, data: {} })).toBe(false);
    expect(isActivityVisible({ type: 2 })).toBe(false);
    expect(isActivityVisible({ type: "2", data: { promotionDesc: "满10减5" } })).toBe(true);
});
