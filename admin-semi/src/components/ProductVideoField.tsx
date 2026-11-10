import { useState } from 'react';
import { Button } from '@douyinfe/semi-ui';
import { IconVideo } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import GalleryPickerModal from './GalleryPickerModal';

interface ProductVideoFieldProps {
  value: string;
  onChange: (url: string) => void;
}

export default function ProductVideoField({ value, onChange }: ProductVideoFieldProps) {
  const { t } = useI18n();
  const [pickerOpen, setPickerOpen] = useState(false);

  return (
    <div className="product-video-field">
      {value ? (
        <div className="product-video-preview">
          <video src={value} controls preload="metadata" />
          <div className="product-video-actions">
            <Button size="small" onClick={() => setPickerOpen(true)}>
              {t('更换')}
            </Button>
            <Button size="small" type="danger" theme="light" onClick={() => onChange('')}>
              {t('移除')}
            </Button>
          </div>
        </div>
      ) : (
        <button
          type="button"
          className="image-uploader-add is-gallery"
          onClick={() => setPickerOpen(true)}
        >
          <IconVideo />
          <span>{t('素材中心')}</span>
        </button>
      )}
      <GalleryPickerModal
        visible={pickerOpen}
        mediaType="video"
        multiple={false}
        onCancel={() => setPickerOpen(false)}
        onConfirm={(urls) => {
          onChange(urls[0] || '');
          setPickerOpen(false);
        }}
      />
    </div>
  );
}
