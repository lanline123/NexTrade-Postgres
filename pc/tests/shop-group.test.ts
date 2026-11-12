import assert from "node:assert/strict";
import test from "node:test";
import { groupLinkUrl, groupListParams } from "../utils/shop-group.ts";

test("多选分组跳转到对应列表并去重", () => {
    assert.equal(groupLinkUrl({ path: "shopGroup", groupIds: [5, 8, 5] }), "/shop/list?groupIds=5,8");
    assert.equal(groupLinkUrl({ path: "storeGroup", groupIds: [12, 15] }), "/store/list?groupIds=12,15");
});

test("兼容历史单选及嵌套配置", () => {
    assert.equal(groupLinkUrl({ path: "shopGroup", id: 5 }), "/shop/list?groupIds=5");
    assert.equal(groupLinkUrl({ path: "storeGroup", data: { id: 8 } }), "/store/list?groupIds=8");
    assert.equal(groupLinkUrl({ path: "shopGroup", data: { groupIds: [5, 8] } }), "/shop/list?groupIds=5,8");
});

test("空选择不生成全量链接", () => {
    assert.equal(groupLinkUrl({ path: "shopGroup", groupIds: [] }), "");
    assert.equal(groupLinkUrl({ path: "shopGroup", groupIds: [0, -1, "abc", 1.5] }), "");
});

test("请求保留类型与分组条件，旧入口不增加筛选", () => {
    assert.deepEqual(groupListParams({ groupIds: "5,8" }, 1), { groupIds: "5,8", shopType: 1 });
    assert.deepEqual(groupListParams({ groupIds: ["12", "15"] }, 2), { groupIds: "12,15", shopType: 2 });
    assert.deepEqual(groupListParams({}, 1), {});
});

test("显式无效条件留给后端校验，不能退化为全量请求", () => {
    assert.deepEqual(groupListParams({ groupIds: "" }, 1), { groupIds: "", shopType: 1 });
    assert.deepEqual(groupListParams({ groupIds: "abc" }, 2), { groupIds: "abc", shopType: 2 });
});
