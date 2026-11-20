import { useCallback, useEffect, useRef, useState, type ChangeEvent } from 'react';
import { Button, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost, apiUpload, isDemo } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

export interface DeliveryFileItem {
  fileId: number;
  productId: number;
  fileName: string;
  fileExt: string;
  fileSize: number;
  addTime: number;
}

/** 字节数转可读大小（B / KB / MB / GB，保留一位小数）。 */
export function formatFileSize(bytes: number): string {
  const size = Number(bytes || 0);
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`;
  return `${(size / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

const ACCEPT_EXTS =
  '.zip,.rar,.7z,.tar,.gz,.bz2,.xz,.pdf,.doc,.docx,.xls,.xlsx,.ppt,.pptx,.txt,.md,.csv,.epub,.mp3,.wav,.mp4,.mov,.exe,.msi,.dmg,.pkg,.apk,.iso,.json';

/**
 * 交付附件管理（文件型交付，我方新增）：独立端点保存，不随商品主表单提交
 * （对齐 saleStatus 的侧端点模式）。买家购买后凭短时效令牌下载，
 * 附件永不直链公开。仅编辑态可用，新建商品先保存再添加。
 */
export default function DeliveryFilesField({ productId }: { productId?: number }) {
  const { t } = useI18n();
  const [files, setFiles] = useState<DeliveryFileItem[]>([]);
  const [uploading, setUploading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const load = useCallback(() => {
    if (!productId) return;
    apiGet<DeliveryFileItem[]>('/product/product/deliveryFiles', { productId })
      .then((list) => setFiles(Array.isArray(list) ? list : []))
      .catch(() => undefined);
  }, [productId]);

  useEffect(() => {
    load();
  }, [load]);

  if (!productId) {
    return (
      <Text type="tertiary" size="small">
        {t('保存商品后可添加交付附件。')}
      </Text>
    );
  }

  const onPick = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) return;
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持保存，请登录后操作') });
      return;
    }
    setUploading(true);
    try {
      await apiUpload('/product/product/uploadDeliveryFile', file, {
        productId: String(productId),
      });
      Toast.success({ content: t('附件已上传') });
      load();
    } catch (error) {
      Toast.error({
        content: error instanceof Error ? error.message : t('附件上传失败'),
      });
    } finally {
      setUploading(false);
    }
  };

  const remove = async (item: DeliveryFileItem) => {
    try {
      await apiPost('/product/product/delDeliveryFile', { id: item.fileId });
      Toast.success({ content: t('附件已删除') });
      load();
    } catch (error) {
      Toast.error({
        content: error instanceof Error ? error.message : t('附件删除失败'),
      });
    }
  };

  return (
    <div style={{ width: 420 }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {files.map((item) => (
          <div
            key={item.fileId}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              border: '1px solid var(--semi-color-border)',
              borderRadius: 4,
              padding: '6px 10px',
            }}
          >
            <span
              style={{
                flex: 1,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                fontSize: 13,
              }}
              title={item.fileName}
            >
              {item.fileName}
            </span>
            <Text type="tertiary" size="small">
              {formatFileSize(item.fileSize)}
            </Text>
            <Button size="small" type="danger" theme="borderless" onClick={() => remove(item)}>
              {t('删除')}
            </Button>
          </div>
        ))}
        {!files.length && (
          <Text type="tertiary" size="small">
            {t('暂无附件')}
          </Text>
        )}
      </div>
      <input ref={inputRef} type="file" accept={ACCEPT_EXTS} style={{ display: 'none' }} onChange={onPick} />
      <Button
        size="small"
        loading={uploading}
        style={{ marginTop: 8 }}
        onClick={() => inputRef.current?.click()}
      >
        {t('上传附件')}
      </Button>
      <div style={{ marginTop: 6 }}>
        <Text type="tertiary" size="small">
          {t('支持压缩包、文档、电子书、音视频与常见安装包，单个不超过 200MB，每件商品最多 10 个。买家购买后凭限时链接下载。')}
        </Text>
      </div>
    </div>
  );
}
