import { useEffect, useMemo, useRef, useState } from 'react';
import { Button, Empty, Input, Modal, Pagination, Select, Spin, Toast } from '@douyinfe/semi-ui';
import { IconGallery, IconSearch } from '@douyinfe/semi-icons';
import { apiGet, apiUpload, isDemo } from '../api';
import { useI18n } from '../i18n';
import './gallery-picker.css';

interface GalleryPic {
  picId?: number;
  galleryId?: number;
  picName?: string;
  picUrl?: string;
  picThumb?: string;
}

interface RawGalleryNode {
  id?: number;
  galleryId?: number;
  parentId?: number;
  name?: string;
  galleryName?: string;
  children?: RawGalleryNode[];
}

interface GalleryPickerModalProps {
  visible: boolean;
  multiple?: boolean;
  limit?: number;
  onCancel: () => void;
  onConfirm: (urls: string[]) => void;
}

const PAGE_SIZE = 30;

const flattenGroups = (
  nodes: RawGalleryNode[],
  depth = 0,
): Array<{ label: string; value: number }> => {
  const options: Array<{ label: string; value: number }> = [];
  (nodes || []).forEach((node) => {
    const id = Number(node.galleryId ?? node.id ?? 0);
    options.push({ label: `${'　'.repeat(depth)}${node.galleryName || node.name || ''}`, value: id });
    if (node.children?.length) options.push(...flattenGroups(node.children, depth + 1));
  });
  return options;
};

const picUrlOf = (pic: GalleryPic) => pic.picUrl || pic.picThumb || '';

export default function GalleryPickerModal({
  visible,
  multiple = true,
  limit,
  onCancel,
  onConfirm,
}: GalleryPickerModalProps) {
  const { t } = useI18n();
  const [keyword, setKeyword] = useState('');
  const [galleryId, setGalleryId] = useState(0);
  const [page, setPage] = useState(1);
  const [items, setItems] = useState<GalleryPic[]>([]);
  const [total, setTotal] = useState(0);
  const [groups, setGroups] = useState<RawGalleryNode[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!visible) return;
    setSelected([]);
    setKeyword('');
    setGalleryId(0);
    setPage(1);
    apiGet<{ records?: RawGalleryNode[] }>('/setting/gallery/tree', { type: 'image' })
      .then((result) => setGroups(result?.records || []))
      .catch(() => undefined);
  }, [visible]);

  useEffect(() => {
    if (!visible || isDemo()) return;
    let alive = true;
    setLoading(true);
    apiGet<any>('/setting/galleryPic/list', {
      page,
      size: PAGE_SIZE,
      galleryId: galleryId || undefined,
      keyword: keyword.trim() || undefined,
      sortOrder: 'desc',
    })
      .then((result) => {
        if (!alive) return;
        const records: GalleryPic[] =
          result?.galleryPicPage?.records || result?.records || result?.list || [];
        setItems(records.filter((item) => picUrlOf(item)));
        setTotal(Number(result?.galleryPicPage?.total ?? result?.total ?? records.length));
      })
      .catch((error) => {
        if (alive) Toast.error({ content: error instanceof Error ? error.message : t('加载素材失败') });
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [visible, page, galleryId, reloadKey, t]);

  const groupOptions = useMemo(
    () => [{ label: t('全部图片'), value: 0 }, ...flattenGroups(groups)],
    [groups, t],
  );

  const toggle = (pic: GalleryPic) => {
    const url = picUrlOf(pic);
    if (!url) return;
    setSelected((current) => {
      if (current.includes(url)) return current.filter((item) => item !== url);
      if (!multiple) return [url];
      if (limit !== undefined && current.length >= limit) {
        Toast.warning({ content: t('最多选择 {count} 张', { count: limit }) });
        return current;
      }
      return [...current, url];
    });
  };

  const upload = async (files: FileList | null) => {
    if (!files?.length) return;
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持上传图片') });
      return;
    }
    setUploading(true);
    try {
      const urls: string[] = [];
      for (const file of Array.from(files)) {
        const result = await apiUpload<{ picUrl?: string }>(
          '/setting/galleryPic/uploadImg?galleryId=0',
          file,
        );
        if (result?.picUrl) urls.push(result.picUrl);
      }
      if (urls.length) {
        setSelected((current) => {
          if (!multiple) return [urls[urls.length - 1]];
          const next = [...current];
          urls.forEach((url) => {
            if (next.length < (limit ?? Number.MAX_SAFE_INTEGER) && !next.includes(url)) next.push(url);
          });
          return next;
        });
        setPage(1);
        setReloadKey((key) => key + 1);
        Toast.success({ content: t('图片上传成功') });
      }
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('图片上传失败') });
    } finally {
      setUploading(false);
      if (fileRef.current) fileRef.current.value = '';
    }
  };

  return (
    <Modal
      className="gallery-picker-modal"
      title={t('从素材中心选择')}
      visible={visible}
      width={860}
      footer={
        <div className="gp-footer">
          <span className="gp-selected">{t('已选 {count} 张', { count: selected.length })}</span>
          <div className="gp-footer-actions">
            <Button onClick={onCancel}>{t('取消')}</Button>
            <Button
              theme="solid"
              type="primary"
              disabled={!selected.length}
              onClick={() => onConfirm(selected)}
            >
              {t('确定')}
            </Button>
          </div>
        </div>
      }
      onCancel={onCancel}
    >
      <div className="gp-toolbar">
        <Button
          icon={<IconGallery />}
          loading={uploading}
          onClick={() => fileRef.current?.click()}
        >
          {t('上传图片')}
        </Button>
        <input
          ref={fileRef}
          type="file"
          accept="image/*"
          multiple
          hidden
          onChange={(event) => void upload(event.target.files)}
        />
        <Input
          className="gp-search"
          prefix={<IconSearch />}
          value={keyword}
          placeholder={t('图片名称')}
          showClear
          onChange={setKeyword}
          onEnterPress={() => {
            setPage(1);
            setReloadKey((key) => key + 1);
          }}
        />
        <Select
          className="gp-group"
          value={galleryId}
          optionList={groupOptions}
          onChange={(value) => {
            setGalleryId(Number(value));
            setPage(1);
          }}
        />
      </div>
      <Spin spinning={loading}>
        {items.length ? (
          <div className="gp-grid">
            {items.map((pic, index) => {
              const url = picUrlOf(pic);
              const isSelected = selected.includes(url);
              return (
                <div
                  key={`${pic.picId ?? index}-${url}`}
                  className={`gp-item${isSelected ? ' is-selected' : ''}`}
                  title={pic.picName || ''}
                  onClick={() => toggle(pic)}
                >
                  <img src={pic.picThumb || url} alt={pic.picName || ''} />
                  {isSelected && <span className="gp-check">✓</span>}
                  {pic.picName && <span className="gp-name">{pic.picName}</span>}
                </div>
              );
            })}
          </div>
        ) : (
          !loading && <Empty description={t('图库暂无图片')} />
        )}
      </Spin>
      {total > PAGE_SIZE && (
        <div className="gp-pagination">
          <Pagination
            total={total}
            pageSize={PAGE_SIZE}
            currentPage={page}
            onPageChange={(next) => setPage(next)}
          />
        </div>
      )}
    </Modal>
  );
}
