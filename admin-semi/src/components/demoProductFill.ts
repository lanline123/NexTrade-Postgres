/**
* 演示商品真实化填充器（用户口径 2026-11-21）
*
* 背景：各装修模块 defaults 内置演示商品（productId=0 的编造快照：假名称/
* 假价格/演示图），源自原版 defaults 契约。用户 2026-11-21 定总口径：
* 所有模块的演示商品一律随机取自自有商城现有在售商品，不再编造。
*
* 机制：模块新建/插入时（H5 编辑器 ModuleToolbar 添加 + 模板套用、
* PC 编辑器 addModule/addModuleAt），对新实例数据深遍历，把演示条目
* 按原位顺序逐个换成随机抽到的真实商品记录（管理端商品列表原始记录，
* 与商品选择器 ProductInfoSelect 写入页面的快照同形状）；抽样池不足时
* 剩余演示位直接移除（诚实空位），取数失败/商城无在售商品时全部移除，
* 绝不把编造商品写进页面数据。
*
* 抽样口径：在售（productStatus=1）且审核通过（checkStatus=1）的商品
* 池（管理端列表第 1 页 size=100）内 Fisher-Yates 洗牌后按 productId
* 去重取用；每次插入重新取池重新抽，模块内不重复（池小于需求量时
* 只填池量、余位移除）。分组型模块（多组演示列表）全模块共用一次
* 抽样、跨组不重复。
*
* 演示条目判定：对象形态、productName 为非空字符串、且 productId 非
* 正数。空占位条目（如 PC 营销通栏卡位空槽 productName 为空）不命中，
* 不受影响；带 productSn 的存量快照不在本机制处理范围（仅新建实例）。
*/
import { apiGet } from "../api";

const isDemoEntry = (x: any): boolean =>
!!x &&
typeof x === "object" &&
!Array.isArray(x) &&
typeof x.productName === "string" &&
x.productName!== "" &&
!(Number(x.productId?? x.id) > 0);

/** 深遍历收集全部演示条目所在数组与下标（按遍历顺序） */
function collectDemoSlots(root: any, slots: Array<{ arr: any[]; index: number}>): void {
if (Array.isArray(root)) {
root.forEach((item, index) => {
if (isDemoEntry(item)) slots.push({ arr: root, index});
else collectDemoSlots(item, slots);
});
return;
}
if (root && typeof root === "object") {
Object.values(root).forEach((v) => collectDemoSlots(v, slots));
}
}

/** 拉取在售商品池并洗牌（去重后返回原始记录数组）；失败返回空数组 */
async function fetchShuffledOnSalePool(): Promise<Array<Record<string, any>>> {
try {
const result = await apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>>}>(
"/product/product/list",
{ productStatus: 1, checkStatus: 1, page: 1, size: 100}
);
const rows = (result.records || result.list || []).filter((r) => Number(r?.productId) > 0);
const seen = new Set<number>();
const pool = rows.filter((r) => {
const id = Number(r.productId);
if (seen.has(id)) return false;
seen.add(id);
return true;
});
for (let i = pool.length - 1; i > 0; i--) {
const j = Math.floor(Math.random() * (i + 1));
[pool[i], pool[j]] = [pool[j], pool[i]];
}
return pool;
} catch {
return [];
}
}

/**
* 就地填充模块数据中的演示商品（mutates data 并返回同一对象）。
* 无演示条目时零请求、原样返回。
*/
export async function fillDemoProducts<T extends Record<string, any>>(data: T): Promise<T> {
const slots: Array<{ arr: any[]; index: number}> = [];
collectDemoSlots(data, slots);
if (!slots.length) return data;
const pool = await fetchShuffledOnSalePool();
// 先替换（下标不变），余位按数组分组从后往前移除，避免下标漂移
const removals = new Map<any[], number[]>();
slots.forEach((slot, i) => {
const real = pool[i];
if (real) {
slot.arr[slot.index] = {...real};
} else {
const list = removals.get(slot.arr) || [];
list.push(slot.index);
removals.set(slot.arr, list);
}
});
removals.forEach((indexes, arr) => {
indexes
.sort((a, b) => b - a)
.forEach((index) => arr.splice(index, 1));
});
return data;
}
