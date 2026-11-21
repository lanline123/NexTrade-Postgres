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
*
* 二期（B-3 收口，2026-11-21 用户拍板一并改掉）：编辑器 compat
* Preview 内部写死的兜底演示清单（productA1/productSlide/
* limitedSpecial/newcomerCombo/groupon/PC 商品分组）同样改走本池。
* fetchOnSaleProductPool 为填充器与 Preview 侧共用的唯一取池函数，
* 不另写第二套取数/洗牌逻辑；Preview 侧纯显示层（usePreviewDemoProducts
* 或直接调用取池函数），无来源/无结果时异步取池随机展示，池空/失败
* 显示诚实空态，不落盘、不改任何数据契约。
*
* 三期（PC 拼团收口，2026-11-21 用户拍板同法收口）：PC 拼团三份
* Preview（decoratePc/modules/pc-home-groupon、decoratePc/modules/
* pcGroupon、decoratePc/previews/PcGrouponPreview）未选活动时的写死
* mock 商品清单同改走本池：仅列表为空时经 usePreviewDemoProducts
* 异步取池补足展示，拼团字段映射与 H5 拼团定案一致（主价取真实拼团
* 价字段、无则真实售价，划线价即真实售价），池空/失败不补、列表为
* 空，纯显示层不落盘。
*
* 四期（个数推广，2026-11-21 用户指令）：以新人二重礼为样板，把
* 「默认带随机真实商品」按模块版式定最佳个数推广到整个编辑器——见
* 下方 DEMO_FILL_TARGETS 定案表。fillDemoProducts 增可选 moduleType
* 参数：快照型先按演示条目形状克隆补足到每组目标数再逐位替换；来源型
* （productSelectType=1 指定商品且 productIds 为空）把随机在售商品 id
* 写入 productIds 使模块创建瞬间即带真实商品（画布真实渲染/真实取数
* 画布与店面同源受益）。仍严守红线：只作用于新建/套用模板/恢复默认
* 时刻，绝不触碰用户已选商品与存量页面数据；池不足按池量缩减、取数
* 失败宁缺勿造；同模块内快照位与来源 id 共用一池、不重复抽同一商品。
*/
import { useEffect, useState } from "react";
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

/**
* 拉取在售商品池并洗牌（去重后返回原始记录数组）；失败返回空数组。
* 填充器与编辑器 Preview 兜底共用的唯一取池函数（见文件头二期说明）。
*/
export async function fetchOnSaleProductPool(): Promise<Array<Record<string, any>>> {
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

/* ------------------------------------------------------------------ */
/* 四期定案：逐模块最佳演示个数（用户口径，按版式定）                    */
/* list   = 快照型：每个演示数组的目标条数（分组模块即每组条数）         */
/* source = 来源型：创建时写入 productIds 的随机在售商品数              */
/* 固定卡位类（营销通栏恒 2 卡位、PC 主推大图恒 2 槽位）不消费商品      */
/* 列表，不在此表、个数不动。依据逐行注释，详见本轮报告总表。            */
/* ------------------------------------------------------------------ */
export interface DemoFillTarget {
list?: number;
source?: number;
}

export const DEMO_FILL_TARGETS: Record<string, DemoFillTarget> = {
/* H5 快照型（个数 = 原版 defaults 条数 = 画布完整形态，本轮登记定案） */
customProduct: { list: 4 }, // 个性商品：海报 + 2×2 网格；设置选品上限即 4，不可超
comboSuggestions: { list: 4 }, // 组合推荐：每组横排每屏 2 件 × 2 屏
countdown: { list: 4 }, // 倒计时：横滑行一屏 + 滑出感
groupSliding: { list: 5 }, // 分组滑动：原版每组 5 件
specialOffer: { list: 4 }, // 特价：2×2 完整形态
productGrouping: { list: 6 }, // 商品分组：每组横排每屏 2 件 × 3 屏
mixedProductDisplay: { list: 4 }, // 商品混排：每组每屏 2 件 × 2 屏
productCarousel: { list: 4 }, // 商品轮播：一圈轮播 + 切换余量
/* H5 来源型（现状默认 0 件，本轮起创建即带） */
product: { source: 6 }, // 商品：样式1 单列 6 行 / 样式2 两列 3 行 / 样式3 三列 2 行
product2: { source: 6 }, // 商品2（跨境版）：两列 3 行
productA1: { source: 6 }, // 商品A1：原版演示列表即 6 件（与二期 Preview 兜底同数）
productSlide: { source: 4 }, // 商品滑动：42% 宽横滑，一屏 2 件 + 滑出第二屏
limitedSpecial: { source: 4 }, // 限时特惠：30% 宽横滑，3 件露头 + 第 4 件滑出
newcomerCombo: { source: 4 }, // 新人二重礼（用户点名样板）：28% 宽横滑，同上口径
/* PC 快照型 */
"pc-product-group": { list: 6 }, // PC 商品分组：每组 6 件（与二期 Preview 兜底同数）
/* PC 来源型 */
"pc-product": { source: 8 }, // PC 商品：默认样式1 四列 × 2 行
"pc-recommend-product": { source: 10 }, // PC 推荐商品：5 列 × 默认 2 行（defaultRows）
pcCatProductSimple: { source: 8 }, // 分类商品-简：其 productNumber 即 8
};

/** Preview 兜底取数：与创建填充共用同一张定案表，避免两处口径漂移。 */
export function demoTargetCount(moduleType: string): number {
const t = DEMO_FILL_TARGETS[moduleType];
return t?.source ?? t?.list ?? 0;
}

/** 活动类 Preview 演示倒计时的展示用结束时刻（仅预览渲染、永不落数据）。 */
export function demoEndTimeString(hours = 24): string {
const d = new Date(Date.now() + hours * 3600 * 1000);
const p = (n: number) => String(n).padStart(2, "0");
return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** 快照补足：按数组把演示条目克隆（深拷贝该组首条形状）补到每组目标数。 */
function padDemoSlots(root: any, perList: number): void {
const slots: Array<{ arr: any[]; index: number}> = [];
collectDemoSlots(root, slots);
const byArr = new Map<any[], Array<{ arr: any[]; index: number}>>();
slots.forEach((s) => {
const group = byArr.get(s.arr) || [];
group.push(s);
byArr.set(s.arr, group);
});
byArr.forEach((group, arr) => {
if (group.length >= perList) return;
const shape = arr[group[0].index];
for (let n = group.length; n < perList; n++) {
arr.push(JSON.parse(JSON.stringify(shape)));
}
});
}

/** 来源配置收集：指定商品（productSelectType=1）且 productIds 尚无真实 id 的配置对象。 */
function collectSourceConfigs(root: any, out: Array<Record<string, any>>): void {
if (Array.isArray(root)) {
root.forEach((item) => collectSourceConfigs(item, out));
return;
}
if (root && typeof root === "object") {
const ids = (root as any).productIds;
if (
Number((root as any).productSelectType) === 1 &&
Array.isArray(ids) &&
!ids.some((id: any) => Number(id) > 0)
) {
out.push(root);
}
Object.values(root).forEach((v) => collectSourceConfigs(v, out));
}
}

/**
* 就地填充模块数据中的演示商品（mutates data 并返回同一对象）。
* 无演示条目且无待填来源配置时零请求、原样返回。
* moduleType 传入时按 DEMO_FILL_TARGETS 定案个数补足/写入（四期）。
*/
export async function fillDemoProducts<T extends Record<string, any>>(data: T, moduleType?: string): Promise<T> {
const target = moduleType ? DEMO_FILL_TARGETS[moduleType] : undefined;
// 快照型先补足到每组目标数，补出的克隆位随即一并被真实商品逐位替换
if (target?.list) padDemoSlots(data, target.list);
const slots: Array<{ arr: any[]; index: number}> = [];
collectDemoSlots(data, slots);
const sourceConfigs: Array<Record<string, any>> = [];
if (target?.source) collectSourceConfigs(data, sourceConfigs);
if (!slots.length && !sourceConfigs.length) return data;
const pool = await fetchOnSaleProductPool();
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
// 来源配置：从池中快照未消费部分顺序取 id 写入（同模块共用一池、不重复）；
// 池已空则保持空 productIds——宁缺勿造，用户在设置中选品后即有商品。
let cursor = slots.length;
sourceConfigs.forEach((cfg) => {
const ids: Array<string | number> = [];
while (ids.length < (target?.source ?? 0) && cursor < pool.length) {
ids.push(pool[cursor].productId);
cursor += 1;
}
if (ids.length) cfg.productIds = ids;
});
return data;
}

/**
* Preview 兜底演示商品 hook（纯显示层，不落盘）：
* enabled 为真（模块无来源/来源无结果）时异步取在售池，随机取前
* count 条返回；取池中/池为空/取数失败均返回空数组，由 Preview 按
* 各自既有空态渲染（诚实空态），绝不回退写死假商品。
*/
export function usePreviewDemoProducts(enabled: boolean, count: number): Array<Record<string, any>> {
const [items, setItems] = useState<Array<Record<string, any>>>([]);
useEffect(() => {
if (!enabled || count <= 0) {
setItems([]);
return;
}
let alive = true;
fetchOnSaleProductPool().then((pool) => {
if (alive) setItems(pool.slice(0, count));
});
return () => {
alive = false;
};
}, [enabled, count]);
return items;
}
