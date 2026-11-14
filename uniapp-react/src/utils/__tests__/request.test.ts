import { beforeEach, describe, expect, it, vi } from "vitest";

import { __setRequestImpl } from "../../../tests/mocks/taro";
import request from "../request";
import { configureRequestRuntime } from "../requestRuntime";

/**
 * 等价迁移自旧 utils/__tests__/request.test.ts（describe
 * "request envelope and cancellation contract"）：
 * 信封解包、业务错误 reject、401 并发只初始化一次、取消语义。
 * React 版 request 走 @tarojs/taro 桩 + configureRequestRuntime 注入运行时，
 * 断言口径与旧版一致（旧版 uni.request 桥在此对应 Taro.request 桩）。
 */

const runtime = {
    getClientType: () => "h5",
    getLocaleCode: () => "zh-CN",
    handleLogin: vi.fn(),
    initialize: vi.fn(async () => {})
};

function mockResponse(data: any) {
    __setRequestImpl((options: any) => {
        options.success({ data, statusCode: 200 });
        return { abort: () => options.fail?.({ errMsg: "request:fail abort" }) };
    });
}

describe("request envelope and cancellation contract", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        configureRequestRuntime(runtime as any);
    });

    it("code=0 时解包 data 返回", async () => {
        mockResponse({ code: 0, data: { hello: "world" }, message: "ok" });
        await expect(request({ url: "common/config/base", method: "get" })).resolves.toEqual({ hello: "world" });
    });

    it("业务错误码原样 reject 响应体", async () => {
        mockResponse({ code: 500, data: null, message: "boom" });
        await expect(request({ url: "x", method: "get" })).rejects.toMatchObject({ code: 500, message: "boom" });
    });

    it("401 触发登录处理且并发只初始化一次；noSkipLogin 时不触发", async () => {
        mockResponse({ code: 401, data: null, message: "unauthorized" });
        await expect(request({ url: "a", method: "get" })).rejects.toBeTruthy();
        await expect(request({ url: "b", method: "get" })).rejects.toBeTruthy();
        expect(runtime.handleLogin).toHaveBeenCalledTimes(2);
        expect(runtime.initialize).toHaveBeenCalledTimes(1);

        vi.clearAllMocks();
        await expect(request({ url: "c", method: "get", noSkipLogin: true } as any)).rejects.toBeTruthy();
        expect(runtime.handleLogin).not.toHaveBeenCalled();
    });

    it("请求被 abort 时以 code=-1 取消语义 reject", async () => {
        __setRequestImpl((options: any) => {
            const task = {
                abort: () => options.fail({ errMsg: "request:fail abort" })
            };
            return task;
        });
        const { abortRequest } = await import("../request");
        const pending = request({ url: "slow", method: "get", requestKey: "k1", cancelPrevious: false } as any);
        expect(abortRequest("k1")).toBe(true);
        await expect(pending).rejects.toMatchObject({ code: -1 });
    });
});
