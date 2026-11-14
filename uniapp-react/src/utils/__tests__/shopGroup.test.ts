import { describe, expect, it } from "vitest";

import { decodeGroupQuery, groupListParams, validGroupParams } from "../shopGroup";

/**
 * 等价迁移自旧 utils/__tests__/shopGroup.test.ts（被测 decodeGroupQuery），
 * 并按 React 版同文件的扩展导出（groupListParams/validGroupParams）补齐断言。
 */
describe("decodeGroupQuery", () => {
    it("解码一层 URI 编码的搜索关键词", () => {
        expect(decodeGroupQuery(encodeURIComponent("锂电池 48V"))).toBe("锂电池 48V");
    });

    it("普通字符串原样返回，非法编码不抛错", () => {
        expect(decodeGroupQuery("abc")).toBe("abc");
        expect(decodeGroupQuery("100%")).toBe("100%");
        expect(decodeGroupQuery(undefined)).toBe("");
        expect(decodeGroupQuery(null)).toBe("");
    });
});

describe("groupListParams / validGroupParams", () => {
    it("显式 groupIds 优先且保留 shopType", () => {
        expect(groupListParams({ groupIds: "1%2C2" }, 1)).toEqual({ groupIds: "1,2", shopType: 1 });
        expect(groupListParams({ groupId: "7" }, 2)).toEqual({ groupId: "7", shopType: 2 });
        expect(groupListParams({}, 1)).toEqual({ shopType: 1 });
    });

    it("合法性校验：逗号分隔正整数且不越界", () => {
        expect(validGroupParams({})).toBe(true);
        expect(validGroupParams({ groupIds: "1,2,3" })).toBe(true);
        expect(validGroupParams({ groupIds: "0" })).toBe(false);
        expect(validGroupParams({ groupIds: "abc" })).toBe(false);
        expect(validGroupParams({ groupId: "2147483648" })).toBe(false);
    });
});
