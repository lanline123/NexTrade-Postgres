/**
 * 商品详情 HTML 格式转换（tigshop 存储格式 ⇄ wangEditor 可编辑格式）。
 *
 * 存储格式（后端 product_desc 列、ProductDescDTO 约定）：
 * - 内容按块组织，块间以 `<div data-division=1></div>` 分隔；
 * - 图片块是 `<div class="desc-pic-item"><img src="..."></div>`；
 * - 其余块是普通 HTML（p/span/strong/列表等），按 text 块原样渲染。
 * 后端读取时用 desc_arr_from_html 按上述规则切块给前台渲染。
 *
 * wangEditor（Slate）的 HTML 解析会丢弃一切 <div> 包裹的内容（只剩空段落），
 * 老格式描述进编辑器会被整体吞掉、保存即清空。这里在抽屉加载/提交两端做转换：
 * - 加载：图片块 → `<p><img src="..."></p>`，分隔符去掉，文本块原样；
 * - 提交：仅含一张图的段落 → 图片块，其余块原样，块间补回分隔符。
 * 两端互逆，未编辑直接保存时内容等价往返（块数、图片、文本均不变）。
 */

const DIVISION = '<div data-division=1></div>';
const DIVISION_RE = /<div[^>]*data-division[^>]*>\s*<\/div>/gi;

function extractImgSrc(html: string): string {
  const match = html.match(/<img[^>]*\ssrc="([^"]*)"/i) || html.match(/<img[^>]*\ssrc='([^']*)'/i);
  return match ? match[1] : '';
}

function picBlock(src: string): string {
  return `<div class="desc-pic-item"><img src="${src}"></div>`;
}

function isEmptyParagraph(html: string): boolean {
  return /^<p>(\s|<br\s*\/?>)*<\/p>$/i.test(html.trim());
}

/** 存储格式 → 编辑器格式（ProductFormDrawer 加载详情时调用）。 */
export function descHtmlToEditorHtml(stored: string): string {
  if (!stored || !stored.trim()) return '';
  const out: string[] = [];
  for (const raw of stored.split(DIVISION_RE)) {
    const chunk = raw.trim();
    if (!chunk) continue;
    if (chunk.includes('desc-pic-item')) {
      const src = extractImgSrc(chunk);
      if (src) out.push(`<p><img src="${src}"></p>`);
      continue;
    }
    out.push(chunk);
  }
  return out.join('');
}

/** 编辑器格式 → 存储格式（ProductFormDrawer 提交时调用）。 */
export function editorHtmlToDescHtml(editorHtml: string): string {
  if (!editorHtml || !editorHtml.trim()) return '';
  const doc = new DOMParser().parseFromString(editorHtml, 'text/html');
  const blocks: string[] = [];
  doc.body.childNodes.forEach((node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = (node.textContent || '').trim();
      if (text) blocks.push(`<p>${text}</p>`);
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;
    const el = node as Element;
    if (el.tagName === 'IMG') {
      const src = el.getAttribute('src');
      if (src) blocks.push(picBlock(src));
      return;
    }
    const imgs = el.querySelectorAll('img');
    const text = (el.textContent || '').replace(/[\s ]/g, '');
    if (el.tagName === 'P' && imgs.length === 1 && text === '') {
      const src = imgs[0].getAttribute('src');
      if (src) {
        blocks.push(picBlock(src));
        return;
      }
    }
    blocks.push(el.outerHTML);
  });
  // 编辑器首尾的空段落是占位产物，去掉；中间的空段落保留（原描述的空行）。
  while (blocks.length && isEmptyParagraph(blocks[0])) blocks.shift();
  while (blocks.length && isEmptyParagraph(blocks[blocks.length - 1])) blocks.pop();
  return blocks.join(DIVISION);
}
