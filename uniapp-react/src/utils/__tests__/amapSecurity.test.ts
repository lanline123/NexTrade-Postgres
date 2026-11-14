import { afterEach, describe, expect, it } from "vitest";

import { applyAmapH5Config } from "../amapSecurity";

/**
 * 等价迁移自旧 utils/__tests__/amapSecurity.test.ts（被测 applyAmapH5Config）。
 * 旧版断言 uni-h5 的 __uniConfig 注入；React 版同签名改为写入
 * window._AMapSecurityConfig，断言随实现形态等价调整。
 */
describe("applyAmapH5Config", () => {
    afterEach(() => {
        delete (window as any)._AMapSecurityConfig;
        delete (window as any).__TIG_AMAP_KEY__;
    });

    // 实现仅在 TARO_ENV=h5 时生效（测试环境默认非 h5，逐用例显式设置）
    process.env.TARO_ENV = "h5";

    it("有安全密钥时写入 securityJsCode", () => {
        applyAmapH5Config("key-1", "secret-1");
        expect((window as any)._AMapSecurityConfig).toEqual(
            expect.objectContaining({ securityJsCode: "secret-1" })
        );
    });

    it("无密钥时回落本站代理服务路径，且不抛错", () => {
        applyAmapH5Config("key-1", "");
        const config = (window as any)._AMapSecurityConfig;
        expect(config?.serviceHost || config?.securityJsCode === "").toBeTruthy();
        expect(() => applyAmapH5Config()).not.toThrow();
        expect(() => applyAmapH5Config(null, null)).not.toThrow();
    });
});
