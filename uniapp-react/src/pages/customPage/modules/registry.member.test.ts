import { describe, expect, it } from "vitest";

import { MODULE_REGISTRY } from "./registry";

/**
 * 会员中心模块注册完整性（2026-11-19 个人中心页 10012 真实渲染空白回归）：
 * 编辑器真实渲染与首页/自定义页共用同一分发器，模块类型必须在本表注册，
 * 否则 decoratePreview 一律跳过、画布空白。此测试只查注册表本身
 * （懒加载组件在查表阶段不执行 loader，不拉起会员组件的重依赖）。
 */
const MEMBER_TYPES_CAMEL = [
    "userInfo",
    "userInfoV2",
    "userInfo3",
    "orderEntry",
    "orderEntryV2",
    "myWallet",
    "myWalletV2",
    "adImage",
    "myService",
    "account",
    "fixedFunctionalArea"
] as const;

const MEMBER_ALIASES: Record<string, string> = {
    "user-info": "userInfo",
    "user-info-v2": "userInfoV2",
    "user-info3": "userInfo3",
    "order-entry": "orderEntry",
    "order-entry-v2": "orderEntryV2",
    "my-wallet": "myWallet",
    "my-wallet-v2": "myWalletV2",
    "ad-image": "adImage",
    "my-service": "myService",
    "fixed-functional-area": "fixedFunctionalArea"
};

/** 10012 个人中心页已发布数据的真实模块类型（MCP decorate_get_page 实取） */
const TYPES_10012 = ["userInfo", "orderEntry", "fixedFunctionalArea", "adImage", "myService", "account"];

describe("MODULE_REGISTRY 会员中心模块", () => {
    it("会员模块 camelCase 类型全部已注册", () => {
        for (const type of MEMBER_TYPES_CAMEL) {
            expect(MODULE_REGISTRY[type], type).toBeDefined();
        }
    });

    it("10012 的 6 个模块类型全部已注册（真实渲染不再跳过）", () => {
        for (const type of TYPES_10012) {
            expect(MODULE_REGISTRY[type], type).toBeDefined();
        }
    });

    it("kebab-case 别名与 camelCase 共用同一组件实例（MemberDecorate 口径）", () => {
        for (const [alias, camel] of Object.entries(MEMBER_ALIASES)) {
            expect(MODULE_REGISTRY[alias], alias).toBe(MODULE_REGISTRY[camel]);
        }
    });

    it("会员模块不与首页既有模块串位（adImage ≠ imageAd、myService 独立注册）", () => {
        expect(MODULE_REGISTRY.adImage).not.toBe(MODULE_REGISTRY.imageAd);
        expect(MODULE_REGISTRY.myService).toBeDefined();
        expect(MODULE_REGISTRY.account).toBeDefined();
    });
});
