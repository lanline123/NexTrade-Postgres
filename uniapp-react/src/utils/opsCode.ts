import { getOpsCode } from "@/api/common";

/**
 * 运营代码注入（后台「运营代码与验证」，论坛帖 477914 批次）：
 * app 启动拉取一次公开配置，把平台管理员填写的第三方代码注入页面——
 * - headCode → document.head（尽早执行）
 * - bodyCode → document.body（统计代码位：百度统计/CNZZ/GA 等）
 * - wapKefuJavascript → document.body（移动端客服 JS，原版客服设置字段，
 *   与内置客服并行生效、互不排斥）
 *
 * 仅 H5 环境有 document；小程序/App 端无 DOM 注入面，直接跳过。
 * 代码由平台管理员粘贴、原样执行（写入端点后端挂 platform_only，
 * XSS 边界 = 平台管理员权限），不做转义/过滤。全部为空（未配置）
 * 时零注入零影响；拉取失败静默（运营代码缺失不阻塞商城启动）。
 * SPA 单次启动只注入一次（模块级标记），避免重复挂载统计/客服脚本。
 */

let injected = false;

/** 把代码片段注入目标节点：<script> 必须以真实元素重建才会执行
 * （innerHTML 塞入的 script 不执行），其余节点（noscript/像素图/
 * 厂商 div）原样移入。 */
const injectSnippet = (code: string | undefined, target: HTMLElement | null) => {
    const raw = (code || "").trim();
    if (!raw || !target) return;
    const tpl = document.createElement("template");
    tpl.innerHTML = raw;
    const nodes = Array.from(tpl.content.childNodes);
    for (const node of nodes) {
        if (node.nodeName === "SCRIPT") {
            const source = node as HTMLScriptElement;
            const script = document.createElement("script");
            for (const attr of Array.from(source.attributes)) {
                script.setAttribute(attr.name, attr.value);
            }
            if (source.textContent) script.textContent = source.textContent;
            target.appendChild(script);
        } else {
            target.appendChild(node);
        }
    }
};

/** app 启动拉取运营代码并注入（重复调用只执行一次）。 */
export const fetchOpsCode = async (): Promise<void> => {
    if (injected) return;
    injected = true;
    if (typeof document === "undefined") return;
    try {
        const result: any = await getOpsCode();
        const data: Record<string, string> = result?.data ?? result ?? {};
        injectSnippet(data.headCode, document.head);
        injectSnippet(data.bodyCode, document.body);
        injectSnippet(data.wapKefuJavascript, document.body);
    } catch {
        // 拉取失败不注入、不阻塞启动（与 fetchSystemMode 同口径）
    }
};
