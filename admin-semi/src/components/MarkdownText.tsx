/**
 * MarkdownText（通用组件）：把纯文本按 Markdown 基础子集渲染为 React 元素树。
 * 首个调用点：AI 助手对话页（助手消息 + 流式气泡）。
 *
 * 安全口径：全程只构造 React 文本/元素节点，绝不使用 dangerouslySetInnerHTML，
 * 原文中的 HTML 一律按普通文本显示；链接仅放行 http/https/mailto，其余协议只显示文字。
 *
 * 支持子集（与主流 Markdown 常用基础语法对齐）：
 * - 块级：段落、# ~ ###### 标题、无序列表（- * +）、有序列表（1. / 1)）、
 *   ``` 围栏代码块、> 引用、--- / *** / ___ 分隔线。
 * - 行内：**加粗** / __加粗__、*斜体* / _斜体_、***粗斜体***、`行内代码`、[文字](链接)。
 * 强调一律成对解析：未闭合、内容贴空白（如乘号 3 * 5、通配 *.png）不触发，
 * 按原文显示；下划线强调另守「词内不触发」，snake_case 这类标识符不误判。
 * 不在子集内（登记不做）：GFM 表格、嵌套列表、图片、脚注、删除线、HTML 直嵌。
 */
import { Fragment, type CSSProperties, type ReactNode } from 'react';

const pStyle: CSSProperties = { margin: '0 0 6px' };
const headingStyles: Record<number, CSSProperties> = {
  1: { fontSize: 16, fontWeight: 600, margin: '8px 0 4px' },
  2: { fontSize: 15, fontWeight: 600, margin: '8px 0 4px' },
  3: { fontSize: 14, fontWeight: 600, margin: '6px 0 4px' },
  4: { fontSize: 14, fontWeight: 600, margin: '6px 0 4px' },
  5: { fontSize: 14, fontWeight: 600, margin: '6px 0 4px' },
  6: { fontSize: 14, fontWeight: 600, margin: '6px 0 4px' },
};
const listStyle: CSSProperties = { margin: '4px 0 6px', paddingLeft: 20 };
const liStyle: CSSProperties = { margin: '2px 0' };
const preStyle: CSSProperties = {
  background: 'var(--semi-color-fill-1)',
  borderRadius: 6,
  padding: '10px 12px',
  overflowX: 'auto',
  fontSize: 12.5,
  lineHeight: 1.55,
  margin: '4px 0 8px',
  whiteSpace: 'pre',
};
const inlineCodeStyle: CSSProperties = {
  fontFamily: 'SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace',
  background: 'var(--semi-color-fill-1)',
  borderRadius: 4,
  padding: '1px 4px',
  fontSize: '0.92em',
};
const quoteStyle: CSSProperties = {
  borderLeft: '3px solid var(--semi-color-border)',
  paddingLeft: 10,
  margin: '4px 0 8px',
  color: 'var(--semi-color-text-2)',
};
const hrStyle: CSSProperties = {
  border: 'none',
  borderTop: '1px solid var(--semi-color-border)',
  margin: '8px 0',
};
const linkStyle: CSSProperties = { color: 'var(--semi-color-link)' };

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
        emit(<code style={inlineCodeStyle}>{code}</code>);
        i = end + 1;
        continue;
      }
    }

    if (ch === '[') {
      const m = LINK_RE.exec(text.slice(i));
      if (m) {
        if (isSafeUrl(m[2])) {
          emit(
            <a href={m[2]} target="_blank" rel="noreferrer" style={linkStyle}>
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
        <pre key={key++} style={preStyle}>
          <code>{buf.join('\n')}</code>
        </pre>,
      );
      continue;
    }

    const heading = HEADING_RE.exec(line);
    if (heading) {
      const level = heading[1].length;
      const Tag = `h${level}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';
      out.push(
        <Tag key={key++} style={headingStyles[level]}>
          {parseInline(heading[2])}
        </Tag>,
      );
      i += 1;
      continue;
    }

    if (HR_RE.test(line)) {
      out.push(<hr key={key++} style={hrStyle} />);
      i += 1;
      continue;
    }

    if (QUOTE_RE.test(line)) {
      const buf: string[] = [];
      while (i < lines.length && QUOTE_RE.test(lines[i])) {
        buf.push(lines[i].replace(/^\s*>\s?/, ''));
        i += 1;
      }
      out.push(
        <blockquote key={key++} style={quoteStyle}>
          {parseInline(buf.join('\n'))}
        </blockquote>,
      );
      continue;
    }

    if (UL_RE.test(line)) {
      const items: string[] = [];
      while (i < lines.length && UL_RE.test(lines[i])) {
        items.push(lines[i].replace(UL_RE, ''));
        i += 1;
      }
      out.push(
        <ul key={key++} style={listStyle}>
          {items.map((item, idx) => (
            <li key={idx} style={liStyle}>
              {parseInline(item)}
            </li>
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
        <ol key={key++} style={listStyle}>
          {items.map((item, idx) => (
            <li key={idx} style={liStyle}>
              {parseInline(item)}
            </li>
          ))}
        </ol>,
      );
      continue;
    }

    const buf: string[] = [line];
    i += 1;
    while (i < lines.length && lines[i].trim() && !startsBlock(lines[i])) {
      buf.push(lines[i]);
      i += 1;
    }
    out.push(
      <p key={key++} style={pStyle}>
        {parseInline(buf.join('\n'))}
      </p>,
    );
  }
  return out;
}

export default function MarkdownText({ text }: { text: string }) {
  return <div>{parseBlocks(text)}</div>;
}
