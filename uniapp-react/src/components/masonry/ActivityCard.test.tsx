import { render } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import ActivityCard from "./ActivityCard";

vi.mock("@tarojs/components", () => ({ Text: "span", View: "div" }));
vi.mock("@/utils/i18n", () => ({ t: (value: string) => value }));

it("renders no digits for a zero activity type and keeps a valid zero coupon description", () => {
    const { container, rerender } = render(<ActivityCard activityInfo={{ type: 0 }} />);
    expect(container.textContent).toBe("");
    rerender(<ActivityCard activityInfo={{ type: 2, data: { promotionDesc: "0元券" } }} />);
    expect(container.textContent).toBe("券0元券");
});
