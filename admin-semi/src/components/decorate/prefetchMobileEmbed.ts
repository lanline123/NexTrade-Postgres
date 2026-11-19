/**
 * H5 编辑器 iframe 入口预热（2026-11-19，我方新增）
 *
 * 背景：H5 装修编辑器的真实渲染画布是 iframe 嵌入前台 /mobile/ 应用
 * （见 decorateEditorH5/components/canvasBridge.ts 的 EMBED_URL），冷启动要
 * 下载并启动整个 Taro 入口包（量测约 684 KB，gzip 约 211 KB）。编辑器侧已有
 * 「加载门」把这段等待盖住（editor-loading-gate 批），本模块再往前一步：
 * 管理员打开装修页面管理列表（PC / H5 任一）时，在浏览器空闲时后台预取
 * /mobile/ 入口资源，之后点进 H5 编辑器时 iframe 直接命中缓存。
 *
 * 口径：
 * - 只预取静态入口资源：/mobile/ 的 index.html 本身 + 解析其 HTML 引用的
 *   入口 js/css（哈希命名、已配 immutable 长缓存，预取收益最大）。
 * - 解析失败（跨源读不到 HTML 等）就只预取 index.html，全程静默，
 *   绝不报错打断列表页；不预取任何接口、不带凭证（fetch credentials=omit）。
 * - 同会话只跑一次（模块级标记）；requestIdleCallback 调度，
 *   不支持时降级 setTimeout，不阻塞列表渲染与交互。
 */

/** 前台根地址：与 decorateEditorH5/components/canvasBridge.ts 的 EMBED_ORIGIN 保持一致 */
const MOBILE_ROOT = "https://battery.recy.cc/mobile/";

let scheduled = false;
const injected = new Set<string>();

function addPrefetch(url: string): void {
    if (injected.has(url)) return;
    injected.add(url);
    try {
        const link = document.createElement("link");
        link.rel = "prefetch";
        link.href = url;
        if (/\.js(?:[?#]|$)/i.test(url)) link.as = "script";
        else if (/\.css(?:[?#]|$)/i.test(url)) link.as = "style";
        document.head.appendChild(link);
    } catch {
        /* 预取是纯优化：注入失败静默 */
    }
}

/** 从 index.html 文本里摘出入口 js/css 的绝对 URL（src/href 属性，相对路径按 /mobile/ 解析） */
function extractEntryAssets(html: string): string[] {
    const urls = new Set<string>();
    const re = /(?:src|href)\s*=\s*["']([^"']+)["']/gi;
    let m: RegExpExecArray | null;
    while ((m = re.exec(html)) !== null) {
        const raw = m[1];
        if (!/\.(?:js|css)(?:[?#]|$)/i.test(raw)) continue;
        try {
            urls.add(new URL(raw, MOBILE_ROOT).toString());
        } catch {
            /* 单个坏 URL 跳过 */
        }
    }
    return [...urls];
}

async function warm(): Promise<void> {
    addPrefetch(MOBILE_ROOT);
    try {
        const res = await fetch(MOBILE_ROOT, { credentials: "omit" });
        if (!res.ok) return;
        const html = await res.text();
        for (const url of extractEntryAssets(html)) addPrefetch(url);
    } catch {
        /* 读不到 HTML（跨源策略等）时只保留 index.html 预取，不重试不报错 */
    }
}

/** 在浏览器空闲时调度一次预热；同会话重复调用无副作用 */
export function scheduleMobileEmbedPrefetch(): void {
    if (scheduled || typeof window === "undefined") return;
    scheduled = true;
    const run = () => {
        void warm();
    };
    const ric = (
        window as unknown as {
            requestIdleCallback?: (cb: () => void, opts?: { timeout: number }) => number;
        }
    ).requestIdleCallback;
    if (typeof ric === "function") {
        ric(run, { timeout: 4000 });
    } else {
        window.setTimeout(run, 1500);
    }
}
