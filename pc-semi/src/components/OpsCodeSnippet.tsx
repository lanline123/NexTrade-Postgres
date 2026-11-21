/**
 * 运营代码片段渲染（服务端组件）：把后台「运营代码与验证」的代码位
 * （headCode / bodyCode / kefuJavascript）以真实 <script> 元素渲染进
 * 首屏 HTML——统计/客服代码 SSR 直出，不等 hydration，首访不漏统计。
 *
 * 片段拆分规则见 `@/lib/ops-code` 的 parseCodeSnippet：script 元素逐个
 * 渲染（src 外链或内联正文 + 原属性），其余 HTML（noscript/像素图等）
 * 走一个 display:contents 的容器（不影响布局）。代码为空时渲染 null。
 * 代码由平台管理员粘贴、原样执行，不转义（见 lib/ops-code 头注）。
 */
import { parseCodeSnippet } from '@/lib/ops-code';

export default function OpsCodeSnippet({ code }: { code?: string | null }) {
  const { scripts, html } = parseCodeSnippet(code);
  if (!scripts.length && !html) return null;
  return (
    <>
      {scripts.map((spec, i) => {
        const { src, content, attrs } = spec;
        const props: Record<string, unknown> = { ...attrs };
        // React  DOM 属性名归一（其余属性含 data-* 原样透传）
        if (typeof props.crossorigin === 'string') {
          props.crossOrigin = props.crossorigin;
          delete props.crossorigin;
        }
        if (typeof props.referrerpolicy === 'string') {
          props.referrerPolicy = props.referrerpolicy;
          delete props.referrerpolicy;
        }
        return src ? (
          <script key={i} src={src} {...props} />
        ) : (
          <script key={i} {...props} dangerouslySetInnerHTML={{ __html: content || '' }} />
        );
      })}
      {html ? (
        <div style={{ display: 'contents' }} suppressHydrationWarning dangerouslySetInnerHTML={{ __html: html }} />
      ) : null}
    </>
  );
}
