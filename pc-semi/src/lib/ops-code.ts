/**
 * 运营代码与验证（后台「设置 → SEO → 运营代码与验证」，论坛帖 477914
 * 批次）：PC 前台消费侧。
 *
 * 后端公开端点 `GET /api/common/config/opsCode` 下发平台管理员配置的：
 * - 站长验证：六平台 meta 验证码（verifyGoogle/verifyBing/verifyBaidu/
 *   verify360/verifySogou/verifyShenma）+ 验证文件（verifyFileName/
 *   verifyFileContent，文件托管在 middleware，见 middleware.ts）；
 * - 第三方代码：headCode（头部）/ bodyCode（底部，统计代码位）；
 * - 客服 JS：kefuJavascript（PC 端；移动端 wapKefuJavascript 由 H5 消费）。
 *
 * 全部为空（未配置）时本模块所有消费点零输出、零影响。代码由平台
 * 管理员粘贴、原样注入执行（XSS 边界 = 平台管理员权限，写入端点
 * 后端挂 platform_only），本模块不做转义/过滤——转义即破坏代码位
 * 语义。与 `getThemeSettings` 同口径：300s ISR 缓存、失败返回 null。
 */

const SERVER_API_BASE = (process.env.PC_API_BASE || 'https://battery.recy.cc').replace(/\/$/, '');

export interface OpsCodeSettings {
  verifyGoogle?: string;
  verifyBing?: string;
  verifyBaidu?: string;
  verify360?: string;
  verifySogou?: string;
  verifyShenma?: string;
  verifyFileName?: string;
  verifyFileContent?: string;
  headCode?: string;
  bodyCode?: string;
  kefuJavascript?: string;
  wapKefuJavascript?: string;
}

/** 服务端取运营代码与验证配置（RootLayout / generateMetadata 共用）。 */
export async function getOpsCodeSettings(): Promise<OpsCodeSettings | null> {
  try {
    const response = await fetch(`${SERVER_API_BASE}/api/common/config/opsCode`, {
      next: { revalidate: 300 },
      signal: AbortSignal.timeout(3000),
    });
    if (!response.ok) return null;
    const json: any = await response.json().catch(() => null);
    const data = json?.data ?? json ?? null;
    return data && typeof data === 'object' ? (data as OpsCodeSettings) : null;
  } catch {
    return null;
  }
}

const nonEmpty = (v?: string): string | undefined => {
  const s = (v || '').trim();
  return s ? s : undefined;
};

/**
 * 站长验证 meta（供 RootLayout 的 generateMetadata）：
 * Google 走 Next 原生 `verification.google`（渲染 google-site-verification），
 * 其余五平台走 `verification.other` 自定义名。全部未配置时返回
 * undefined，generateMetadata 不输出任何 verification 字段。
 */
export function opsVerification(ops: OpsCodeSettings | null):
  | { google?: string; other?: Record<string, string> }
  | undefined {
  if (!ops) return undefined;
  const other: Record<string, string> = {};
  const bing = nonEmpty(ops.verifyBing);
  if (bing) other['msvalidate.01'] = bing;
  const baidu = nonEmpty(ops.verifyBaidu);
  if (baidu) other['baidu-site-verification'] = baidu;
  const v360 = nonEmpty(ops.verify360);
  if (v360) other['360-site-verification'] = v360;
  const sogou = nonEmpty(ops.verifySogou);
  if (sogou) other['sogou_site_verification'] = sogou;
  const shenma = nonEmpty(ops.verifyShenma);
  if (shenma) other['shenma-site-verification'] = shenma;
  const google = nonEmpty(ops.verifyGoogle);
  if (!google && Object.keys(other).length === 0) return undefined;
  return {
    ...(google ? { google } : {}),
    ...(Object.keys(other).length ? { other } : {}),
  };
}

export interface OpsScriptSpec {
  /** 外部脚本地址（与 content 二选一） */
  src?: string;
  /** 内联脚本正文 */
  content?: string;
  /** 其余属性（async/defer 为布尔 true，其余为字符串） */
  attrs: Record<string, string | boolean>;
}

const SCRIPT_RE = /<script\b([^>]*)>([\s\S]*?)<\/script\s*>/gi;
const ATTR_RE = /([a-zA-Z_:][-a-zA-Z0-9_:.]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
const BOOLEAN_ATTRS = new Set(['async', 'defer', 'nomodule']);

/**
 * 把管理员粘贴的代码片段拆成「脚本清单 + 其余 HTML」：
 * React 直接渲染含 <script> 的 innerHTML 时脚本不会执行，必须把
 * script 元素逐个拆出、以真实元素渲染（SSR 首屏即在 HTML 里）；
 * 其余内容（noscript / 统计像素 img / 厂商 div 等）走 innerHTML。
 */
export function parseCodeSnippet(code?: string | null): { scripts: OpsScriptSpec[]; html: string } {
  const raw = (code || '').trim();
  if (!raw) return { scripts: [], html: '' };
  const scripts: OpsScriptSpec[] = [];
  const html = raw
    .replace(SCRIPT_RE, (_m, attrText: string, body: string) => {
      const attrs: Record<string, string | boolean> = {};
      let src: string | undefined;
      ATTR_RE.lastIndex = 0;
      let am: RegExpExecArray | null;
      while ((am = ATTR_RE.exec(attrText)) !== null) {
        const name = am[1];
        const value = am[2] ?? am[3] ?? am[4];
        if (name.toLowerCase() === 'src') {
          src = value || '';
        } else if (value === undefined && BOOLEAN_ATTRS.has(name.toLowerCase())) {
          attrs[name] = true;
        } else {
          attrs[name] = value ?? '';
        }
      }
      const content = (body || '').trim();
      if (src) scripts.push({ src, attrs });
      else if (content) scripts.push({ content, attrs });
      return '';
    })
    .trim();
  return { scripts, html };
}
