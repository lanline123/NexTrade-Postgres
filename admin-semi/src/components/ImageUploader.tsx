import { useRef, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import { IconClose, IconGallery, IconPlus } from '@douyinfe/semi-icons';
import { apiUpload, isDemo } from '../api';
import { useI18n } from '../i18n';
import GalleryPickerModal from './GalleryPickerModal';

interface ImageUploaderProps {
  value: string[];
  onChange: (next: string[]) => void;
  max?: number;
  single?: boolean;
  /** 显示「素材中心」入口，从图库选择已有图片 */
  gallery?: boolean;
}

interface UploadResult {
  picUrl: string;
  picThumb?: string;
}

export default function ImageUploader({
  value,
  onChange,
  max = 9,
  single = false,
  gallery = false,
}: ImageUploaderProps) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const limit = single ? 1 : max;
  const remaining = limit - value.length;

  const pick = () => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持上传图片') });
      return;
    }
    inputRef.current?.click();
  };

  const upload = async (files: FileList | null) => {
    if (!files || !files.length) return;
    const remainingSlots = limit - value.length;
    if (remainingSlots <= 0) {
      Toast.warning({ content: t('最多上传 {count} 张图片', { count: limit }) });
      return;
    }
    setUploading(true);
    try {
      const next = [...value];
      for (const file of Array.from(files).slice(0, remainingSlots)) {
        const result = await apiUpload<UploadResult>('/setting/galleryPic/uploadImg?galleryId=0', file);
        if (result?.picUrl) next.push(result.picUrl);
      }
      onChange(next);
      Toast.success({ content: t('图片上传成功') });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('图片上传失败') });
    } finally {
      setUploading(false);
      if (inputRef.current) inputRef.current.value = '';
    }
  };

  const remove = (index: number) => {
    onChange(value.filter((_, i) => i !== index));
  };

  const move = (from: number, to: number) => {
    if (from === to || from < 0 || to < 0 || from >= value.length || to >= value.length) return;
    const next = [...value];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    onChange(next);
  };

  const resetDrag = () => {
    setDragIndex(null);
    setOverIndex(null);
  };

  return (
    <div className="image-uploader">
      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        multiple
        hidden
        onChange={(event) => upload(event.target.files)}
      />
      <div className="image-uploader-list">
        {value.map((url, index) => (
          <div
            className={[
              'image-uploader-item',
              !single ? 'is-sortable' : '',
              index === 0 ? 'is-main' : '',
              dragIndex === index ? 'is-dragging' : '',
              overIndex === index && dragIndex !== index ? 'is-drop-target' : '',
            ]
              .filter(Boolean)
              .join(' ')}
            key={`${url}-${index}`}
            draggable={!single}
            onDragStart={(event) => {
              if (single) return;
              setDragIndex(index);
              event.dataTransfer.effectAllowed = 'move';
              event.dataTransfer.setData('text/plain', String(index));
            }}
            onDragOver={(event) => {
              if (single) return;
              event.preventDefault();
              event.dataTransfer.dropEffect = 'move';
              setOverIndex(index);
            }}
            onDrop={(event) => {
              if (single) return;
              event.preventDefault();
              const from = dragIndex ?? Number(event.dataTransfer.getData('text/plain'));
              move(from, index);
              resetDrag();
            }}
            onDragEnd={resetDrag}
          >
            <img src={url} alt={t('图片 {index}', { index: index + 1 })} draggable={false} />
            {!single && index === 0 && <span className="image-main-badge">{t('主图')}</span>}
            <div className="image-uploader-actions">
              <button
                type="button"
                className="is-danger"
                title={t('删除')}
                aria-label={t('删除')}
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  remove(index);
                }}
              >
                <IconClose size="small" />
              </button>
            </div>
          </div>
        ))}
        {value.length < limit && (
          <>
            <button type="button" className="image-uploader-add" onClick={pick} disabled={uploading}>
              <IconPlus />
              <span>{uploading ? t('上传中...') : t('上传图片')}</span>
            </button>
            {gallery && (
              <button
                type="button"
                className="image-uploader-add is-gallery"
                onClick={() => setPickerOpen(true)}
              >
                <IconGallery />
                <span>{t('素材中心')}</span>
              </button>
            )}
          </>
        )}
      </div>
      <div className="image-uploader-tip">
        {single
          ? t('支持 JPG / PNG / WebP，单张图片。')
          : t('支持 JPG / PNG / WebP，最多 {count} 张；拖拽可调整顺序，第一张自动作为商品主图。', {
              count: max,
            })}
      </div>
      {gallery && (
        <GalleryPickerModal
          visible={pickerOpen}
          multiple={!single}
          limit={Math.max(remaining, 0)}
          onCancel={() => setPickerOpen(false)}
          onConfirm={(urls) => {
            const next = [...value];
            urls.forEach((url) => {
              if (next.length < limit && !next.includes(url)) next.push(url);
            });
            onChange(next);
            setPickerOpen(false);
          }}
        />
      )}
    </div>
  );
}
