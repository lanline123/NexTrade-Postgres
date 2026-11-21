/**
 * GuideMarkdown（PC 导购专用）：把导购助手回复按 Markdown 渲染为 React 元素树。
 *
 * 派生自后台通用组件 MarkdownText（通用组件登记册 #17）的解析与安全口径，
 * 并扩展 GFM 表格：导购回复常以表格做商品对比（2026-11-21 现网实测：真实回复
 * 含 5 列 GFM 表格），而面板原先纯文本裸渲染，竖线/分隔行原样显示、比例字体
 * 下各列无法对齐（用户实报「错版」），故本组件专为导购面补齐表格渲染。
 *
 * 安全口径（与 #17 一致）：全程只构造 React 文本/元素节点，绝不使用
 * dangerouslySetInnerHTML，原文中的 HTML 一律按普通文本显示；链接仅放行
 * http/https/mailto，其余协议只显示文字。
 *
 * 支持子集：段落、# ~ ###### 标题、无序/有序列表、``` 围栏代码块、> 引用、
 * 分隔线、GFM 表格（表头 + 分隔行对齐 + 单元格行内格式）；行内：**加粗** /
 * __加粗__、*斜体* / _斜体_、***粗斜体***、`行内代码`、[文字](链接)。
 * 强调成对解析规则与 #17 相同（未闭合/贴空白不触发，snake_case 不误判）。
 * 不在子集内：图片（按「! + 链接」降解为文字与链接，不生成 <img>）、嵌套列表、
 * 脚注、删除线、HTML 直嵌。
 *
 * 样式走 guide-chat.scss 的 .guide-md 作用域（元素选择器 + 表格类），
 * 表格外层 .guide-md-tablewrap 横向滚动，宽表不撑破 420 抽屉气泡。
 */
import { Fragment, type CSSProperties, type ReactNode } from 'react';

function isSpace(ch: string | undefined): boolean {
  return ch === undefined || /\s/.test(ch);
}

function isAlnum(ch: string | undefined): boolean {
  return ch !== undefined && /[A-Za-z0-9]/.test(ch);
}

function isSafeUrl(url: string): boolean {
  return /^(https?:\/\/|mailto:)/i.test(url);
}

/**
 * 校验一对强调标记是否成立：内容非空、两端不贴空白；
 * 下划线另守词内不触发（openStart/closeEnd 为标记在原文中的起止下标）。
 */
function emphasisOk(
  text: string,
  openStart: number,
  closeEnd: number,
  marker: string,
  inner: string,
): boolean {
  if (!inner || !inner.trim()) return false;
  if (isSpace(inner[0]) || isSpace(inner[inner.length - 1])) return false;
  if (marker === '_') {
    if (isAlnum(text[openStart - 1])) return false;
    if (isAlnum(text[closeEnd])) return false;
  }
  return true;
}

/** 找双标记（** / __）的闭合位置，from 为内容起始下标；无合法闭合返回 -1。 */
function findDoubleClose(text: string, from: number, marker: string): number {
  const delim = marker + marker;
  let j = text.indexOf(delim, from);
  while (j >= 0) {
    const inner = text.slice(from, j);
    if (emphasisOk(text, from - 2, j + 2, marker, inner)) return j;
    j = text.indexOf(delim, j + 1);
  }
  return -1;
}

/** 找单标记（* / _）的闭合位置，扫描时跳过双标记对；无合法闭合返回 -1。 */
function findSingleClose(text: string, from: number, marker: string): number {
  let j = from;
  while (j < text.length) {
    if (text[j] === marker) {
      if (text[j + 1] === marker) {
        j += 2;
        continue;
      }
      const inner = text.slice(from, j);
      if (emphasisOk(text, from - 1, j + 1, marker, inner)) return j;
    }
    j += 1;
  }
  return -1;
}

const LINK_RE = /^\[([^\]\n]+)\]\(([^)\s]+)\)/;

/** 行内解析：代码跨度优先消费，其后链接、加粗、斜体，未成对标记按原文输出。 */
function parseInline(text: string): ReactNode[] {
  const out: ReactNode[] = [];
  let buf = '';
  let seq = 0;
  let i = 0;
  const flush = () => {
    if (buf) {
      out.push(buf);
      buf = '';
    }
  };
  const emit = (el: ReactNode) => {
    flush();
    out.push(<Fragment key={seq++}>{el}</Fragment>);
  };

  while (i < text.length) {
    const ch = text[i];

    if (ch === '`') {
      const end = text.indexOf('`', i + 1);
      if (end > i + 1) {
        const code = text.slice(i + 1, end);
        emit(<code>{code}</code>);
        i = end + 1;
        continue;
      }
    }

    if (ch === '[') {
      const m = LINK_RE.exec(text.slice(i));
      if (m) {
        if (isSafeUrl(m[2])) {
          emit(
            <a href={m[2]} target="_blank" rel="noreferrer">
              {parseInline(m[1])}
            </a>,
          );
        } else {
          emit(<Fragment>{parseInline(m[1])}</Fragment>);
        }
        i += m[0].length;
        continue;
      }
    }

    if (ch === '*' || ch === '_') {
      // ***粗斜体***
      if (text[i + 1] === ch && text[i + 2] === ch) {
        const close = text.indexOf(ch + ch + ch, i + 3);
        if (close > i + 3) {
          const inner = text.slice(i + 3, close);
          if (emphasisOk(text, i, close + 3, ch, inner)) {
            emit(
              <strong>
                <em>{parseInline(inner)}</em>
              </strong>,
            );
            i = close + 3;
            continue;
          }
        }
      }
      if (text[i + 1] === ch) {
        const close = findDoubleClose(text, i + 2, ch);
        if (close >= 0) {
          emit(<strong>{parseInline(text.slice(i + 2, close))}</strong>);
          i = close + 2;
          continue;
        }
      } else {
        const close = findSingleClose(text, i + 1, ch);
        if (close >= 0) {
          emit(<em>{parseInline(text.slice(i + 1, close))}</em>);
          i = close + 1;
          continue;
        }
      }
    }

    if (ch === '\n') {
      emit(<br />);
      i += 1;
      continue;
    }

    buf += ch;
    i += 1;
  }
  flush();
  return out;
}

/* ---------- GFM 表格 ---------- */

type TableAlign = CSSProperties['textAlign'];

/** 拆一行表格为单元格：处理首尾竖线与 \| 转义。 */
function splitTableRow(line: string): string[] {
  let t = line.trim();
  if (t.startsWith('|')) t = t.slice(1);
  if (t.endsWith('|') && !t.endsWith('\\|')) t = t.slice(0, -1);
  const cells: string[] = [];
  let cur = '';
  for (let i = 0; i < t.length; i += 1) {
    if (t[i] === '\\' && t[i + 1] === '|') {
      cur += '|';
      i += 1;
      continue;
    }
    if (t[i] === '|') {
      cells.push(cur.trim());
      cur = '';
      continue;
    }
    cur += t[i];
  }
  cells.push(cur.trim());
  return cells;
}

/** 分隔行单元格（:--- / :-: / ---: / ---）→ 对齐方式，非分隔行返回 null。 */
function parseSepAlign(cell: string): TableAlign | null {
  const c = cell.trim();
  if (!/^:?-+:?$/.test(c)) return null;
  const left = c.startsWith(':');
  const right = c.endsWith(':');
  if (left && right) return 'center';
  if (right) return 'right';
  if (left) return 'left';
  return undefined;
}

/** 判断 lines[i] 是否为分隔行（全单元格均为分隔单元格）。 */
function isSepRow(line: string): boolean {
  if (!line.includes('-')) return false;
  const cells = splitTableRow(line);
  return cells.length > 0 && cells.every((c) => parseSepAlign(c) !== null);
}

/** 判断 lines[i] 是否为表格表头行：含竖线、下一行为分隔行且列数一致。 */
function isTableStart(lines: string[], i: number): boolean {
  if (i + 1 >= lines.length) return false;
  if (!lines[i].includes('|')) return false;
  if (!isSepRow(lines[i + 1])) return false;
  return splitTableRow(lines[i]).length === splitTableRow(lines[i + 1]).length;
}

/* ---------- 块级解析 ---------- */

const FENCE_RE = /^```(\w*)\s*$/;
const FENCE_END_RE = /^```\s*$/;
const HEADING_RE = /^(#{1,6})\s+(.*)$/;
const HR_RE = /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/;
const QUOTE_RE = /^\s*>/;
const UL_RE = /^\s*[-*+]\s+/;
const OL_RE = /^\s*\d{1,9}[.)]\s+/;

function startsBlock(line: string): boolean {
  return (
    FENCE_RE.test(line.trim()) ||
    HEADING_RE.test(line) ||
    HR_RE.test(line) ||
    QUOTE_RE.test(line) ||
    UL_RE.test(line) ||
    OL_RE.test(line)
  );
}

function parseBlocks(text: string): ReactNode[] {
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const out: ReactNode[] = [];
  let key = 0;
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (!line.trim()) {
      i += 1;
      continue;
    }

    if (FENCE_RE.test(line.trim())) {
      const buf: string[] = [];
      i += 1;
      while (i < lines.length && !FENCE_END_RE.test(lines[i].trim())) {
        buf.push(lines[i]);
        i += 1;
      }
      i += 1; // 收尾围栏（缺失时即文末）
      out.push(
        <pre key={key++}>
          <code>{buf.join('\n')}</code>
        </pre>,
      );
      continue;
    }

    if (isTableStart(lines, i)) {
      const header = splitTableRow(line);
      const aligns = splitTableRow(lines[i + 1]).map(
        (c) => parseSepAlign(c) ?? undefined,
      );
      const rows: string[][] = [];
      i += 2;
      while (i < lines.length && lines[i].trim() && lines[i].includes('|')) {
        const cells = splitTableRow(lines[i]);
        while (cells.length < header.length) cells.push('');
        rows.push(cells.slice(0, header.length));
        i += 1;
      }
      out.push(
        <div key={key++} className="guide-md-tablewrap">
          <table className="guide-md-table">
            <thead>
              <tr>
                {header.map((c, j) => (
                  <th key={j} style={{ textAlign: aligns[j] }}>
                    {parseInline(c)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row, r) => (
                <tr key={r}>
                  {row.map((c, j) => (
                    <td key={j} style={{ textAlign: aligns[j] }}>
                      {parseInline(c)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }

    const heading = HEADING_RE.exec(line);
    if (heading) {
      const level = heading[1].length;
      const Tag = `h${level}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';
      out.push(<Tag key={key++}>{parseInline(heading[2])}</Tag>);
      i += 1;
      continue;
    }

    if (HR_RE.test(line)) {
      out.push(<hr key={key++} />);
      i += 1;
      continue;
    }

    if (QUOTE_RE.test(line)) {
      const buf: string[] = [];
      while (i < lines.length && QUOTE_RE.test(lines[i])) {
        buf.push(lines[i].replace(/^\s*>\s?/, ''));
        i += 1;
      }
      out.push(<blockquote key={key++}>{parseInline(buf.join('\n'))}</blockquote>);
      continue;
    }

    if (UL_RE.test(line)) {
      const items: string[] = [];
      while (i < lines.length && UL_RE.test(lines[i])) {
        items.push(lines[i].replace(UL_RE, ''));
        i += 1;
      }
      out.push(
        <ul key={key++}>
          {items.map((item, idx) => (
            <li key={idx}>{parseInline(item)}</li>
          ))}
        </ul>,
      );
      continue;
    }

    if (OL_RE.test(line)) {
      const items: string[] = [];
      while (i < lines.length && OL_RE.test(lines[i])) {
        items.push(lines[i].replace(OL_RE, ''));
        i += 1;
      }
      out.push(
        <ol key={key++}>
          {items.map((item, idx) => (
            <li key={idx}>{parseInline(item)}</li>
          ))}
        </ol>,
      );
      continue;
    }

    const buf: string[] = [line];
    i += 1;
    while (
      i < lines.length &&
      lines[i].trim() &&
      !startsBlock(lines[i]) &&
      !isTableStart(lines, i)
    ) {
      buf.push(lines[i]);
      i += 1;
    }
    out.push(<p key={key++}>{parseInline(buf.join('\n'))}</p>);
  }
  return out;
}

export default function GuideMarkdown({ text }: { text: string }) {
  return <div className="guide-md">{parseBlocks(text)}</div>;
}
