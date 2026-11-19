/**
 * 交付内容链接抽取（付费内容 type=4 / 虚拟商品 type=2 的交付区专用）。
 *
 * 交付内容以富文本呈现时，其中的 URL 在小程序 RichText 里不可跳转、
 * 文本不可复制（Tigshop 论坛帖 80793616636238732 的原始诉求）。交付区
 * 因此把 URL 抽成独立链接行（打开 / 复制按钮），不依赖富文本锚点。
 *
 * 抽取来源：`<a href="...">` 与正文中的裸 http(s) 链接，去重保序。
 */
export function extractUrls(html: string | null | undefined): string[] {
  if (!html) return [];
  const found: string[] = [];
  const seen = new Set<string>();
  const push = (raw: string) => {
    const url = raw
      .replace(/&amp;/g, '&')
      .replace(/[\])）。，；、！？'"」]+$/u, '')
      .trim();
    if (/^https?:\/\//i.test(url) && !seen.has(url)) {
      seen.add(url);
      found.push(url);
    }
  };
  let m: RegExpExecArray | null;
  const hrefRe = /href\s*=\s*["']([^"']+)["']/gi;
  while ((m = hrefRe.exec(html)) !== null) push(m[1]);
  const bareRe = /https?:\/\/[^\s<>"']+/gi;
  while ((m = bareRe.exec(html)) !== null) push(m[0]);
  return found;
}
