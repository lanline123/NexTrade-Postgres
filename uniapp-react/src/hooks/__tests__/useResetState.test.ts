import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { defaultClone, useResetReactive, useResettableRef } from "../useResetState";

/**
 * 本线新增 hook（自旧 hooks/useResetState.ts 等价移植）的契约测试。
 * 旧版语义：reset 回滚到初始值的 JSON 深克隆，快照与现状态互不污染。
 */
describe("useResetState (旧 useResetReactive / useResettableRef)", () => {
    it("useResetReactive 修改后 reset 回到初始深克隆", () => {
        const { result } = renderHook(() => useResetReactive({ a: 1, nested: { b: 2 } }));
        act(() => {
            result.current[1]({ a: 99, nested: { b: 88 } });
        });
        expect(result.current[0]).toEqual({ a: 99, nested: { b: 88 } });
        act(() => {
            result.current[2]();
        });
        expect(result.current[0]).toEqual({ a: 1, nested: { b: 2 } });
        // 再次修改现状态不应污染初始快照
        act(() => {
            result.current[1]((prev: any) => ({ ...prev, a: 5 }));
        });
        act(() => {
            result.current[2]();
        });
        expect(result.current[0].a).toBe(1);
    });

    it("useResettableRef 的 ref 可变且 reset 回滚", () => {
        const { result } = renderHook(() => useResettableRef({ count: 1 }));
        result.current[0].current.count = 42;
        act(() => {
            result.current[1]();
        });
        expect(result.current[0].current).toEqual({ count: 1 });
    });

    it("defaultClone 为 JSON 深克隆", () => {
        const src = { x: { y: 1 } };
        const cloned = defaultClone(src);
        expect(cloned).toEqual(src);
        expect(cloned).not.toBe(src);
        expect(cloned.x).not.toBe(src.x);
        expect(defaultClone(undefined)).toBeUndefined();
        expect(defaultClone(null)).toBeNull();
    });
});
