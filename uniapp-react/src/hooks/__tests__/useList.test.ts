import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import useList from "../useList";

/**
 * 等价迁移自旧 hooks/__tests__/useList.test.ts（describe
 * "typed list pagination"，旧版用 @vue/test-utils 挂载 composable）。
 * React 版以 @testing-library/react 的 renderHook 等价验证：
 * 分页拉取、触底续页合并、resetState 回滚（对应旧 reset 语义）。
 */
describe("typed list pagination", () => {
    it("immediate 拉取第一页并写入 data/total", async () => {
        const api = vi.fn(async (params: any) => ({
            list: [{ id: 1 }, { id: 2 }],
            total: 3,
            page: params.page
        }));
        const { result } = renderHook(() => useList(api as any, { params: { page: 1, size: 2 }, immediate: true }));
        await waitFor(() => expect(result.current.data.length).toBe(2));
        expect(result.current.total).toBe(3);
        expect(api).toHaveBeenCalledWith(expect.objectContaining({ page: 1, size: 2 }));
    });

    it("loadMore 续页合并，resetState 回到初始", async () => {
        const api = vi.fn(async (params: any) => ({
            list: params.page === 1 ? [{ id: 1 }] : [{ id: 2 }],
            total: 2
        }));
        const { result } = renderHook(() => useList(api as any, { params: { page: 1, size: 1 } }));

        await act(async () => {
            await result.current.getList();
        });
        expect(result.current.data).toEqual([{ id: 1 }]);

        // 触底续页在实现内由 useReachBottom 驱动（测试桩为 noop），
        // 此处以其等价入口 getList({page:2}) 验证续页合并语义
        await act(async () => {
            await result.current.getList({ page: 2 } as any);
        });
        expect(result.current.data).toEqual([{ id: 1 }, { id: 2 }]);

        act(() => {
            result.current.resetState();
        });
        expect(result.current.data).toEqual([]);
        expect(result.current.total).toBe(0);
    });

    it("接口 reject 时 error 被记录且 loading 落下", async () => {
        const api = vi.fn(async () => {
            throw new Error("network down");
        });
        const { result } = renderHook(() => useList(api as any, { params: {} }));
        await act(async () => {
            await result.current.getList().catch(() => {});
        });
        expect(result.current.error).toBeTruthy();
        expect(result.current.isLoading).toBe(false);
    });
});
