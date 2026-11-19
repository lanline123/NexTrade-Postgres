import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import CartSwipe from "./CartSwipe";

vi.mock("@tarojs/components", () => ({ View: "div" }));
afterEach(cleanup);

it("keeps a tap and a vertical scroll independent of the horizontal row actions", () => {
    const onOpenChange = vi.fn();
    const onTabClick = vi.fn();
    const { getByText } = render(
        <>
            <CartSwipe open={false} onOpenChange={onOpenChange} rightAction="删除">商品</CartSwipe>
            <button onClick={onTabClick}>我的</button>
        </>
    );
    const row = getByText("商品");
    const start = { clientX: 100, clientY: 100 };
    const end = { clientX: 72, clientY: 160 };
    fireEvent.touchStart(row, { touches: [start] });
    fireEvent.touchMove(row, { touches: [end] });
    fireEvent.touchEnd(row, { changedTouches: [end] });
    fireEvent.click(getByText("我的"));
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(onTabClick).toHaveBeenCalledOnce();
});

it("opens leftward and closes rightward after passing the threshold", () => {
    const onOpenChange = vi.fn();
    const { getByText, rerender } = render(<CartSwipe open={false} onOpenChange={onOpenChange} rightAction="删除">商品</CartSwipe>);
    const drag = (from: number, to: number) => {
        const row = getByText("商品");
        const end = { clientX: to, clientY: 45 };
        fireEvent.touchStart(row, { touches: [{ clientX: from, clientY: 45 }] });
        fireEvent.touchMove(row, { touches: [end] });
        fireEvent.touchEnd(row, { changedTouches: [end] });
    };
    drag(130, 120);
    expect(onOpenChange).not.toHaveBeenCalled();
    drag(130, 60);
    expect(onOpenChange).toHaveBeenLastCalledWith(true);
    rerender(<CartSwipe open onOpenChange={onOpenChange} rightAction="删除">商品</CartSwipe>);
    drag(60, 130);
    expect(onOpenChange).toHaveBeenLastCalledWith(false);
});
