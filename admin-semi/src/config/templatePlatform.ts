/**
* 装修模板平台（tpl.recy.cc）对接配置与数据层。
*
* 模板平台是独立于本站的目录站：运营在 NexTrade 里做好装修页 → 用页面分享
* 生成链接 → 登录模板平台管理端粘贴链接，平台当场抓取页面数据**快照存档**
* （分享链接 7 天过期与导入无关，导入永不回源）。本站编辑器的「从模板导入」
* 弹窗从这里拉分类与模板列表，选中后取导入包（page.data 为 DecorateVO.data
* 原文），在本站走新建草稿页链（create + saveDraft）落库为未发布新页。
*
* 基址为可配置常量：换域名/本地联调时只改这一处（或部署时注入同名全局变量
* `window.__TEMPLATE_PLATFORM_BASE_URL__` 覆盖，不改代码）。
*
* 对接契约（平台线定版，见 ~/workspace/template-platform/README.md「对接契约」）：
* 统一信封 { code: 0, msg: "success", data}，失败时 code 为 HTTP 状态码、msg 为原因。
* - GET {BASE}/api/categories → data: [{ id, name, sort}]（按 sort 升序）
* - GET {BASE}/api/templates?categoryId=&target=&q= → data: 已上架模板列表
* [{ id, name, categoryId, categoryName, target: "mobile"|"pc", cover,
* description, moduleTypes, moduleCount, updatedAt}]（不含页面数据）
* - GET {BASE}/api/templates/{id}/import → data: 导入包
* { templateId, name, target, source: { decorateId, decorateType, pageType},
* page: { decorateTitle, decorateType, pageType, data: { pageModule, moduleList}}}
*/

declare global {
interface Window {
__TEMPLATE_PLATFORM_BASE_URL__?: string;
}
}

/** 模板平台基址（默认 https://tpl.recy.cc，可配置常量） */
export const TEMPLATE_PLATFORM_BASE_URL: string = (
(typeof window!== 'undefined' && window.__TEMPLATE_PLATFORM_BASE_URL__) ||
'https://tpl.recy.cc'
).replace(/\/+$/, '');

/** 适用端：与平台契约 target 取值一致（mobile = 移动端，pc = PC 端） */
export type TemplateTarget = 'mobile' | 'pc';

export interface TemplateCategory {
id: number;
name: string;
sort?: number;
}

export interface TemplateItem {
id: number;
name: string;
categoryId: number;
categoryName?: string;
target: TemplateTarget;
cover: string;
description?: string;
moduleCount?: number;
}

/** 导入包：page.data 是源站 DecorateVO.data 原文（{ pageModule, moduleList}） */
export interface TemplateImportPackage {
templateId: number;
name: string;
target: TemplateTarget;
source?: { decorateId: number; decorateType: number; pageType: number};
page: {
decorateTitle: string;
decorateType: number;
pageType: number;
data: Record<string, any>;
};
}

async function fetchData<T>(path: string): Promise<T> {
const res = await fetch(`${TEMPLATE_PLATFORM_BASE_URL}${path}`, {
method: 'GET',
credentials: 'omit',
});
const json = await res.json().catch(() => null);
if (json == null) throw new Error('模板平台响应格式异常');
if (Number(json.code)!== 0) {
throw new Error(json.msg || `模板平台请求失败（HTTP ${res.status}）`);
}
return json.data as T;
}

/** 拉取模板分类（弹窗顶部分类排） */
export async function fetchTemplateCategories(): Promise<TemplateCategory[]> {
const data = await fetchData<any[]>('/api/categories');
return (Array.isArray(data)? data: [])
.map((raw) => ({ id: Number(raw?.id), name: String(raw?.name?? '').trim(), sort: raw?.sort}))
.filter((c) => c.id > 0 && c.name);
}

/** 拉取某端模板列表，可按分类/关键词过滤（过滤走平台服务端参数） */
export async function fetchTemplates(
target: TemplateTarget,
categoryId?: number,
q?: string,
): Promise<TemplateItem[]> {
const params = new URLSearchParams({ target});
if (categoryId) params.set('categoryId', String(categoryId));
if (q?.trim()) params.set('q', q.trim());
const data = await fetchData<any[]>(`/api/templates?${params.toString()}`);
return (Array.isArray(data)? data: [])
.map((raw) => ({
id: Number(raw?.id),
name: String(raw?.name?? '').trim() || '未命名模板',
categoryId: Number(raw?.categoryId) || 0,
categoryName: raw?.categoryName,
target: (String(raw?.target) === 'pc'? 'pc': 'mobile') as TemplateTarget,
cover: String(raw?.cover?? '').trim(),
description: raw?.description,
moduleCount: typeof raw?.moduleCount === 'number'? raw.moduleCount: undefined,
}))
.filter((item) => item.id > 0 && item.target === target);
}

/** 拉取导入包（平台快照，含完整页面数据） */
export async function fetchTemplateImport(id: number): Promise<TemplateImportPackage> {
const data = await fetchData<TemplateImportPackage>(`/api/templates/${id}/import`);
if (!data?.page?.data) throw new Error('模板导入包缺少页面数据');
return data;
}
