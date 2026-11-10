import { useState } from 'react';
import { IconClose, IconGallery } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import GalleryPickerModal from './GalleryPickerModal';

interface ImageUploaderProps {
  value: string[];
  onChange: (next: string[]) => void;
  max?: number;
  single?: boolean;
}

export default function ImageUploader({
  value,
  onChange,
  max = 9,
  single = false,
}: ImageUploaderProps) {
  const { t } = useI18n();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const limit = single ? 1 : max;
  const remaining = limit - value.length;

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
          <button
            type="button"
            className="image-uploader-add is-gallery"
            onClick={() => setPickerOpen(true)}
          >
            <IconGallery />
            <span>{t('素材中心')}</span>
          </button>
        )}
      </div>
      <div className="image-uploader-tip">
        {single
          ? t('支持 JPG / PNG / WebP，单张图片。')
          : t('支持 JPG / PNG / WebP，最多 {count} 张；拖拽可调整顺序，第一张自动作为商品主图。', {
              count: max,
            })}
      </div>
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
    </div>
  );
}
