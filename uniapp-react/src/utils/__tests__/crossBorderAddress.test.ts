import { describe, expect, it } from "vitest";

import {
    buildCrossBorderAddressData,
    CROSS_BORDER_COUNTRIES,
    emptyCrossBorderAddressFields,
    formatCrossBorderAddress,
    getCrossBorderCountry,
    normalizeInternationalMobile,
    validateInternationalMobile
} from "../crossBorderAddress";

/**
 * v5.8.62 二期（US/CA）注册表与校验用例 + 既有国家回归。
 * validateInternationalMobile 的 translate 用恒等函数，便于断言文案键。
 */
const identity = (key: string) => key;

describe("crossBorderAddress US/CA 注册", () => {
    it("注册表包含 10 个国家且 US/CA 字段结构正确", () => {
        expect(CROSS_BORDER_COUNTRIES.map((item) => item.code)).toEqual([
            "CN",
            "MY",
            "SG",
            "TH",
            "KH",
            "VN",
            "HK",
            "TW",
            "US",
            "CA"
        ]);
        const us = getCrossBorderCountry("US");
        expect(us.dialCode).toBe("1");
        expect(us.regionFields.map((field) => field.key)).toEqual(["province", "city"]);
        const ca = getCrossBorderCountry("ca");
        expect(ca.code).toBe("CA");
        expect(ca.regionFields[0]?.label).toContain("省");
    });

    it("US ZIP 校验：5 位与 ZIP+4 通过，其余拒绝", () => {
        const pattern = getCrossBorderCountry("US").postcodePattern!;
        expect(pattern.test("10001")).toBe(true);
        expect(pattern.test("10001-1234")).toBe(true);
        expect(pattern.test("1000")).toBe(false);
        expect(pattern.test("10001-123")).toBe(false);
        expect(pattern.test("K1A0B1")).toBe(false);
    });

    it("CA 邮编校验：A1A 1A1（可带空格/短横）通过，其余拒绝", () => {
        const pattern = getCrossBorderCountry("CA").postcodePattern!;
        expect(pattern.test("K1A 0B1")).toBe(true);
        expect(pattern.test("K1A0B1")).toBe(true);
        expect(pattern.test("M5V-3L9")).toBe(true);
        expect(pattern.test("10001")).toBe(false);
        expect(pattern.test("1KA 0B1")).toBe(false);
    });

    it("北美手机号：NANP 10 位规则，US/CA 文案不串国", () => {
        expect(validateInternationalMobile("US", "2025550123", "1", identity)).toBe("");
        expect(validateInternationalMobile("CA", "4165550123", "1", identity)).toBe("");
        expect(validateInternationalMobile("US", "1234567890", "1", identity)).not.toBe("");
        const caError = validateInternationalMobile("CA", "12345", "1", identity);
        expect(caError).toContain("加拿大");
        expect(caError).not.toContain("美国");
        expect(normalizeInternationalMobile("US", "2025550123", "1")).toBe("+12025550123");
        expect(normalizeInternationalMobile("CA", "+1 416-555-0123", "1")).toBe("+14165550123");
    });

    it("US 地址组装：address_data 含州/城市/街道/邮编，主地址取 addressLine1", () => {
        const values = {
            ...emptyCrossBorderAddressFields(),
            province: "California",
            city: "San Francisco",
            addressLine1: "1 Market St",
            addressLineTwo: "Suite 100",
            postcode: "94105"
        };
        const data = buildCrossBorderAddressData("US", values);
        expect(data).toEqual({
            province: "California",
            city: "San Francisco",
            addressLineOne: "1 Market St",
            addressLineTwo: "Suite 100",
            postcode: "94105"
        });
        expect(formatCrossBorderAddress("US", values)).toBe("1 Market St");
        expect(formatCrossBorderAddress("CA", { ...values, addressLine1: "1 Queen St" })).toBe("1 Queen St");
    });

    it("既有国家回归：MY 主地址仍取 addressLine1，CN 取 address", () => {
        const base = emptyCrossBorderAddressFields();
        expect(formatCrossBorderAddress("MY", { ...base, addressLine1: "Jalan 1", address: "X" })).toBe("Jalan 1");
        expect(formatCrossBorderAddress("CN", { ...base, address: "中山路 1 号" })).toBe("中山路 1 号");
        expect(getCrossBorderCountry("XX").code).toBe("CN");
    });
});
