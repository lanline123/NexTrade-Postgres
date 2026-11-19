import { describe, expect, it } from "vitest";

import { configPageSlotIndices, resolveConfigPageBranch } from "./resolve";

/**
 * 整页配置型页型分支判定（2026-11-19 登录/搜索/商品详情真实渲染空白修复）：
 * - pt5/6/8 含对应配置模块时命中整页分支，槽位只有配置模块一个；
 * - 其它页型、配置模块缺失/隐藏时不命中，预览回退通用分发（pt1/2/3 回归面）。
 * 数据样本为 10103/10104/10105 三页已发布 moduleList 的真实形状（MCP 实取）。
 */

/** 10103 登录页（pageType=5）已发布 moduleList */
const MODULE_LIST_10103 = [
    {
        isShow: true,
        label: "登录页",
        type: "mobileLogin",
        url: "mobileLogin",
        moduleIndex: 1795100209900,
        module: { titleColor: "#333333", contentPaddingX: 20, dividerColor: "#eeeeee" }
    }
];

/** 10104 搜索页（pageType=6）已发布 moduleList */
const MODULE_LIST_10104 = [
    {
        isShow: true,
        label: "搜索结果",
        type: "mobileSearchResult",
        url: "mobileSearch",
        moduleIndex: 1795100240403,
        module: { searchResultType: 1, layoutStyle: 1, listStyle: 1, backgroundColor: "#f5f5f5" }
    }
];

/** 10105 商品详情页（pageType=8）已发布 moduleList */
const MODULE_LIST_10105 = [
    {
        isShow: true,
        label: "商品详情A1",
        type: "productDetailA1",
        url: "mobileProductDetail",
        moduleIndex: 1795100272689,
        module: { decorateStyle: "A1", buyBarStyle: 1, priceColor: "#ff4d4f" }
    }
];

describe("resolveConfigPageBranch 整页配置型分支判定", () => {
    it("pt5 命中 mobileLogin，index 与 module 原样带出", () => {
        const branch = resolveConfigPageBranch(5, MODULE_LIST_10103);
        expect(branch).not.toBeNull();
        expect(branch?.pageType).toBe(5);
        expect(branch?.index).toBe(0);
        expect(branch?.type).toBe("mobileLogin");
        expect(branch?.module.titleColor).toBe("#333333");
        expect(configPageSlotIndices(5, MODULE_LIST_10103)).toEqual([0]);
    });

    it("pt6 命中 mobileSearchResult", () => {
        const branch = resolveConfigPageBranch(6, MODULE_LIST_10104);
        expect(branch?.type).toBe("mobileSearchResult");
        expect(branch?.index).toBe(0);
        expect(configPageSlotIndices(6, MODULE_LIST_10104)).toEqual([0]);
    });

    it("pt8 命中 productDetailA1/A2/A3 任一版式", () => {
        expect(resolveConfigPageBranch(8, MODULE_LIST_10105)?.type).toBe("productDetailA1");
        for (const type of ["productDetailA2", "productDetailA3"]) {
            const modules = [{ isShow: true, type, moduleIndex: 1, module: {} }];
            expect(resolveConfigPageBranch(8, modules)?.type).toBe(type);
        }
    });

    it("配置模块不在首位时 index 按实际下标（槽位对应配置模块）", () => {
        const modules = [
            { isShow: true, type: "notice", moduleIndex: 1, module: {} },
            { isShow: true, type: "mobileLogin", moduleIndex: 2, module: {} }
        ];
        expect(resolveConfigPageBranch(5, modules)?.index).toBe(1);
        expect(configPageSlotIndices(5, modules)).toEqual([1]);
    });

    it("pt1/2/3/4 不误入分支（即使混入同名 type 也按页型隔离）", () => {
        for (const pageType of [1, 2, 3, 4, 0]) {
            expect(resolveConfigPageBranch(pageType, MODULE_LIST_10103)).toBeNull();
            expect(resolveConfigPageBranch(pageType, MODULE_LIST_10104)).toBeNull();
            expect(resolveConfigPageBranch(pageType, MODULE_LIST_10105)).toBeNull();
            expect(configPageSlotIndices(pageType, MODULE_LIST_10105)).toBeNull();
        }
    });

    it("页型与模块对不上时不命中（pt5 拿搜索模块、pt8 拿登录模块）", () => {
        expect(resolveConfigPageBranch(5, MODULE_LIST_10104)).toBeNull();
        expect(resolveConfigPageBranch(8, MODULE_LIST_10103)).toBeNull();
        expect(resolveConfigPageBranch(6, MODULE_LIST_10105)).toBeNull();
    });

    it("配置模块缺失时回退（返回 null，预览走通用分发）", () => {
        expect(resolveConfigPageBranch(5, [])).toBeNull();
        expect(resolveConfigPageBranch(6, [{ isShow: true, type: "notice", moduleIndex: 1, module: {} }])).toBeNull();
        expect(resolveConfigPageBranch(8, undefined)).toBeNull();
        expect(resolveConfigPageBranch(8, null)).toBeNull();
    });

    it("配置模块隐藏（isShow=false）时不命中，与既有槽位口径一致", () => {
        const modules = [{ isShow: false, type: "mobileLogin", moduleIndex: 1, module: {} }];
        expect(resolveConfigPageBranch(5, modules)).toBeNull();
        expect(configPageSlotIndices(5, modules)).toBeNull();
    });
});
