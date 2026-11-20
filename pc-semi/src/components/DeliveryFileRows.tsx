'use client';

/**
 * 交付附件行（文件型交付，我方新增）：与 DeliveryLinkRows 同款行样式，
 * 每行呈现附件名 + 大小 + 「下载」。点击先向服务端申领短时效令牌
 * （接口层强校验订单归属与已购/已发货门），再打开一次性下载链接；
 * 链接 30 分钟有效、最多下载 5 次，过期重新点击即可。无附件不渲染。
 */
import { useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import { apiPost } from '@/lib/client-api';

export interface DeliveryFileItem {
  fileId: number;
  productId: number;
  fileName: string;
  fileExt?: string;
  fileSize: number;
}

/** 字节数转可读大小（B / KB / MB / GB，保留一位小数）。 */
export function formatFileSize(bytes: number): string {
  const size = Number(bytes || 0);
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`;
  return `${(size / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

export default function DeliveryFileRows({
  files,
  orderId,
}: {
  files?: DeliveryFileItem[] | null;
  orderId?: number;
}) {
  const { t } = useTranslation();
  const [busyId, setBusyId] = useState(0);
  const list = Array.isArray(files) ? files.filter((f) => f && Number(f.fileId) > 0) : [];
  if (!list.length) return null;

  const download = async (file: DeliveryFileItem) => {
    setBusyId(file.fileId);
    try {
      const grant = await apiPost<{ downloadUrl?: string }>('product/delivery/requestDownload', {
        fileId: file.fileId,
        productId: file.productId,
        orderId: orderId || undefined,
      });
      if (grant?.downloadUrl) {
        window.open(grant.downloadUrl, '_blank');
      } else {
        Toast.error({ content: t('获取下载链接失败，请重试') });
      }
    } catch (error) {
      Toast.error({
        content: error instanceof Error ? error.message : t('获取下载链接失败，请重试'),
      });
    } finally {
      setBusyId(0);
    }
  };

  return (
    <div className="delivery-file-rows" style={{ margin: '8px 0' }}>
      {list.map((file) => (
        <div
          key={file.fileId}
          className="delivery-file-row"
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
          <span
            style={{
              flex: 1,
              minWidth: 0,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            title={file.fileName}
          >
            {file.fileName}
          </span>
          <span style={{ flexShrink: 0, color: 'var(--semi-color-text-2, #999)' }}>
            {formatFileSize(file.fileSize)}
          </span>
          <button
            type="button"
            disabled={busyId === file.fileId}
            style={{
              flexShrink: 0,
              padding: '2px 10px',
              fontSize: 12,
              cursor: 'pointer',
              borderRadius: 10,
              border: '1px solid var(--semi-color-border, #d9d9d9)',
              background: 'var(--semi-color-bg-2, #fff)',
            }}
            onClick={() => void download(file)}
          >
            {busyId === file.fileId ? t('获取中…') : t('下载')}
          </button>
        </div>
      ))}
      <div style={{ fontSize: 12, color: 'var(--semi-color-text-2, #999)' }}>
        {t('下载链接 30 分钟内有效，最多可下载 5 次，过期后重新点击下载即可。')}
      </div>
    </div>
  );
}
