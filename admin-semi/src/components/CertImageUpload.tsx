import { useRef, useState } from 'react';
import { Button, Toast } from '@douyinfe/semi-ui';
import { IconClose } from '@douyinfe/semi-icons';
import { apiUpload, certFileSrc } from '../api';
import { useI18n } from '../i18n';

interface CertImageUploadProps {
  /** 已上传证件的代理路径数组（与 ImageUploader 同形的 string[] 契约）。 */
  value: string[];
  onChange: (next: string[]) => void;
  max?: number;
  single?: boolean;
}

/**
 * 证件图片直传控件（批次B，仿 CertFileUpload 的直传模式）：
 * 本地文件直传 `/setting/config/uploadCertImg` 证件私有通道，
 * 不打开素材中心、不落 gallery_pic；预览经后台鉴权代理出图。
 * 用于证照类表单字段（进件证件照、营业执照等），普通运营图片仍用 ImageUploader。
 */
export default function CertImageUpload({ value, onChange, max = 9, single = false }: CertImageUploadProps) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const limit = single ? 1 : max;

  const upload = async (file: File | undefined) => {
    if (!file) return;
    setUploading(true);
    try {
      const url = await apiUpload<string>('/setting/config/uploadCertImg', file);
      if (!url) throw new Error(t('上传失败'));
      onChange(single ? [url] : [...value, url].slice(0, limit));
      Toast.success({ content: t('上传成功') });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('上传失败') });
    } finally {
      setUploading(false);
      if (inputRef.current) inputRef.current.value = '';
    }
  };

  const remove = (index: number) => {
    onChange(value.filter((_, i) => i !== index));
  };

  return (
    <div className="cert-image-upload" data-widget="cert-image-upload">
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'flex-start' }}>
        {value.map((url, index) => (
          <div key={`${url}-${index}`} style={{ position: 'relative', width: 80, height: 80 }}>
            <img
              src={certFileSrc(url)}
              alt={t('证件图片 {index}', { index: index + 1 })}
              style={{ width: 80, height: 80, objectFit: 'cover', borderRadius: 4 }}
            />
            <button
              type="button"
              aria-label={t('删除')}
              onClick={() => remove(index)}
              style={{
                position: 'absolute',
                top: -6,
                right: -6,
                width: 18,
                height: 18,
                borderRadius: '50%',
                border: 'none',
                background: '#f5222d',
                color: '#fff',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                cursor: 'pointer',
              }}
            >
              <IconClose size="extra-small" />
            </button>
          </div>
        ))}
        {value.length < limit ? (
          <Button loading={uploading} onClick={() => inputRef.current?.click()}>
            {t('点击上传')}
          </Button>
        ) : null}
      </div>
      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        style={{ display: 'none' }}
        onChange={(event) => upload(event.target.files?.[0])}
      />
      <div style={{ color: '#86909c', fontSize: 12, marginTop: 4 }}>
        {t('证件图片直传私有存储，不进入素材中心；支持 JPG / PNG，单张不超过 10MB。')}
      </div>
    </div>
  );
}
