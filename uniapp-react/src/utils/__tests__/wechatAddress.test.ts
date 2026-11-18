import { describe, expect, it } from "vitest";

import {
    findRegionMatch,
    importWechatAddress,
    isWechatAddressImportSupported,
    matchWechatRegionIds,
    normalizeRegionName,
    type FetchRegionChildren,
    type RegionNodeLike
} from "../wechatAddress";

/**
 * 小程序一键获取收货地址（批次D）纯逻辑单测：
 * 地区名归一化、同级名称匹配、省/市/区县逐级匹配（含直辖市跳级、
 * 部分匹配降级），以及非 weapp 环境下导入入口的守卫。
 * chooseAddress 本身需真机/开发者工具授权，本地不可实测，不在此覆盖。
 */
describe("wechatAddress 地区名归一化", () => {
    it("剥离省/市/自治区等后缀", () => {
        expect(normalizeRegionName("广东省")).toBe("广东");
        expect(normalizeRegionName("深圳市")).toBe("深圳");
        expect(normalizeRegionName("内蒙古自治区")).toBe("内蒙古");
        expect(normalizeRegionName("广西壮族自治区")).toBe("广西");
        expect(normalizeRegionName("新疆维吾尔自治区")).toBe("新疆");
        expect(normalizeRegionName("宁夏回族自治区")).toBe("宁夏");
        expect(normalizeRegionName("香港特别行政区")).toBe("香港");
    });

    it("剥离区/县/新区后缀并去空白", () => {
        expect(normalizeRegionName("朝阳区")).toBe("朝阳");
        expect(normalizeRegionName("浦东新区")).toBe("浦东");
        expect(normalizeRegionName(" 平谷县 ")).toBe("平谷");
        expect(normalizeRegionName("大兴安岭地区")).toBe("大兴安岭");
    });

    it("无后缀与空值原样/兜底", () => {
        expect(normalizeRegionName("北京")).toBe("北京");
        expect(normalizeRegionName("")).toBe("");
        expect(normalizeRegionName("区")).toBe("区");
    });
});

describe("wechatAddress 同级名称匹配", () => {
    const nodes: RegionNodeLike[] = [
        { regionId: 11, regionName: "广东" },
        { regionId: 12, regionName: "广西壮族自治区" },
        { regionId: 13, regionName: "湖南省" }
    ];

    it("原名精确命中", () => {
        expect(findRegionMatch(nodes, "广东")?.regionId).toBe(11);
    });

    it("后缀差异经归一命中", () => {
        expect(findRegionMatch(nodes, "广东省")?.regionId).toBe(11);
        expect(findRegionMatch(nodes, "广西")?.regionId).toBe(12);
        expect(findRegionMatch(nodes, "湖南")?.regionId).toBe(13);
    });

    it("找不到返回 undefined", () => {
        expect(findRegionMatch(nodes, "四川省")).toBeUndefined();
        expect(findRegionMatch([], "广东省")).toBeUndefined();
        expect(findRegionMatch(nodes, "")).toBeUndefined();
    });
});

/** 假地区树：root(1) → 省 → 市 → 区县；北京市子级直接是区县（无市级节点） */
const fakeTree: Record<string, RegionNodeLike[]> = {
    "1": [
        { regionId: 101, regionName: "广东省" },
        { regionId: 102, regionName: "北京市" }
    ],
    "101": [
        { regionId: 10101, regionName: "深圳市" },
        { regionId: 10102, regionName: "广州市" }
    ],
    "10101": [
        { regionId: 1010101, regionName: "南山区" },
        { regionId: 1010102, regionName: "福田区" }
    ],
    "102": [
        { regionId: 10201, regionName: "朝阳区" },
        { regionId: 10202, regionName: "海淀区" }
    ]
};
const fakeFetch: FetchRegionChildren = async (parentId) => fakeTree[String(parentId)] || [];

describe("wechatAddress 逐级地区匹配", () => {
    it("普通省/市/区县三级全匹配", async () => {
        const outcome = await matchWechatRegionIds({ provinceName: "广东省", cityName: "深圳市", countyName: "南山区" }, 1, fakeFetch);
        expect(outcome.regionIds).toEqual([101, 10101, 1010101]);
        expect(outcome.regionNames).toEqual(["广东省", "深圳市", "南山区"]);
        expect(outcome.fullyMatched).toBe(true);
    });

    it("微信名称无后缀时也能匹配", async () => {
        const outcome = await matchWechatRegionIds({ provinceName: "广东", cityName: "深圳", countyName: "南山" }, 1, fakeFetch);
        expect(outcome.regionIds).toEqual([101, 10101, 1010101]);
        expect(outcome.fullyMatched).toBe(true);
    });

    it("直辖市省市同名且地区树缺市级节点时跳级匹配区县", async () => {
        const outcome = await matchWechatRegionIds({ provinceName: "北京市", cityName: "北京市", countyName: "朝阳区" }, 1, fakeFetch);
        expect(outcome.regionIds).toEqual([102, 10201]);
        expect(outcome.regionNames).toEqual(["北京市", "朝阳区"]);
        expect(outcome.fullyMatched).toBe(true);
    });

    it("区县匹配失败时保留已匹配前缀并标记未完全匹配", async () => {
        const outcome = await matchWechatRegionIds({ provinceName: "广东省", cityName: "深圳市", countyName: "不存在区" }, 1, fakeFetch);
        expect(outcome.regionIds).toEqual([101, 10101]);
        expect(outcome.fullyMatched).toBe(false);
    });

    it("省级匹配失败时返回空链", async () => {
        const outcome = await matchWechatRegionIds({ provinceName: "四川省", cityName: "成都市", countyName: "锦江区" }, 1, fakeFetch);
        expect(outcome.regionIds).toEqual([]);
        expect(outcome.fullyMatched).toBe(false);
    });

    it("取数抛错时降级为未匹配而非抛错", async () => {
        const failingFetch: FetchRegionChildren = async () => {
            throw new Error("network");
        };
        const outcome = await matchWechatRegionIds({ provinceName: "广东省" }, 1, failingFetch);
        expect(outcome.regionIds).toEqual([]);
        expect(outcome.fullyMatched).toBe(false);
    });
});

describe("wechatAddress 导入守卫", () => {
    it("非 weapp 环境（vitest）不支持导入且主流程返回 unsupported", async () => {
        expect(isWechatAddressImportSupported()).toBe(false);
        const result = await importWechatAddress();
        expect(result.status).toBe("unsupported");
    });
});
