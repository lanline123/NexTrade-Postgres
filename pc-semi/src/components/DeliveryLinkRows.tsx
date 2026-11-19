'use client';

/**
 * 交付链接行（付费内容 / 虚拟内容交付区）：把内容 HTML 里的 URL 抽成
 * 独立一行行链接，每行带「打开」与「复制」——不依赖富文本内锚点的
 * 跳转能力（小程序 RichText 不支持链接跳转与长按复制，见
 * lib/deliveryLinks.ts 的说明）。无 URL 时不渲染。
 */
import { useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { extractUrls } from '@/lib/deliveryLinks';

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      return ok;
    } catch {
      return false;
    }
  }
}

export default function DeliveryLinkRows({ html }: { html?: string | null }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState<string>('');
  const urls = extractUrls(html);
  if (!urls.length) return null;
  return (
    <div className="delivery-link-rows" style={{ margin: '8px 0' }}>
      {urls.map((url) => (
        <div
          key={url}
          className="delivery-link-row"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '6px 10px',
            marginBottom: 6,
            border: '1px solid var(--semi-color-border, #e8e8e8)',
            borderRadius: 6,
            background: 'var(--semi-color-fill-0, #fafafa)',
            fontSize: 13,
          }}
        >
          <a
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            style={{
              flex: 1,
              minWidth: 0,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              color: 'var(--semi-color-link, #0064fa)',
            }}
            title={url}
          >
            {url}
          </a>
          <a
            href={url}
            target="_blank"
            rel="noopener noreferrer"
            style={{ flexShrink: 0, color: 'var(--semi-color-link, #0064fa)' }}
          >
            {t('打开')}
          </a>
          <button
            type="button"
            style={{
              flexShrink: 0,
              padding: '2px 10px',
              fontSize: 12,
              cursor: 'pointer',
              borderRadius: 10,
              border: '1px solid var(--semi-color-border, #d9d9d9)',
              background: 'var(--semi-color-bg-2, #fff)',
            }}
            onClick={() => {
              void copyText(url).then((ok) => {
                if (ok) {
                  setCopied(url);
                  Toast.success(t('链接已复制'));
                } else {
                  Toast.error(t('复制失败，请长按链接复制'));
                }
              });
            }}
          >
            {copied === url ? t('已复制') : t('复制')}
          </button>
        </div>
      ))}
    </div>
  );
}
