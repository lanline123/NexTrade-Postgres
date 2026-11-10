import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import { Button, Empty, Input, Modal, Pagination, Spin, Toast } from '@douyinfe/semi-ui';
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
  picCount?: number;
  children?: RawGalleryNode[];
}

interface GalleryTreeNode {
  id: number;
  name: string;
  count: number;
  children: GalleryTreeNode[];
}

interface GalleryPickerModalProps {
  visible: boolean;
  multiple?: boolean;
  limit?: number;
  onCancel: () => void;
  onConfirm: (urls: string[]) => void;
}

const PAGE_SIZE = 30;

const normalizeTree = (nodes: RawGalleryNode[]): GalleryTreeNode[] =>
  (nodes || []).map((node) => ({
    id: Number(node.galleryId ?? node.id ?? 0),
    name: node.galleryName || node.name || '',
    count: Number(node.picCount ?? 0),
    children: normalizeTree(node.children || []),
  }));

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
  const [expandedIds, setExpandedIds] = useState<number[]>([]);
  const [page, setPage] = useState(1);
  const [items, setItems] = useState<GalleryPic[]>([]);
  const [total, setTotal] = useState(0);
  const [tree, setTree] = useState<GalleryTreeNode[]>([]);
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
    setExpandedIds([]);
    setPage(1);
    apiGet<{ records?: RawGalleryNode[] }>('/setting/gallery/tree', { type: 'image' })
      .then((result) => setTree(normalizeTree(result?.records || [])))
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

  const selectedFolderName = useMemo(() => {
    if (galleryId === 0) return t('全部图片');
    const find = (nodes: GalleryTreeNode[]): string | undefined => {
      for (const node of nodes) {
        if (node.id === galleryId) return node.name;
        const hit = find(node.children);
        if (hit) return hit;
      }
      return undefined;
    };
    return find(tree) || t('全部图片');
  }, [galleryId, tree, t]);

  const toggleExpand = (id: number) => {
    setExpandedIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );
  };

  const selectFolder = (id: number) => {
    setGalleryId(id);
    setPage(1);
  };

  const toggleSelect = (pic: GalleryPic) => {
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
        setGalleryId(0);
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

  const renderTree = (nodes: GalleryTreeNode[], depth = 0) =>
    nodes.map((node) => {
      const hasChildren = node.children.length > 0;
      const expanded = expandedIds.includes(node.id);
      const active = galleryId === node.id;
      return (
        <Fragment key={node.id}>
          <div
            className={`gp-tree-node${active ? ' is-active' : ''}`}
            style={{ paddingLeft: 10 + depth * 16 }}
            onClick={() => selectFolder(node.id)}
            title={node.name}
          >
            <span
              className={`gp-tree-arrow${hasChildren ? '' : ' is-leaf'}${expanded ? ' is-expanded' : ''}`}
              onClick={(event) => {
                event.stopPropagation();
                if (hasChildren) toggleExpand(node.id);
              }}
            />
            <span className="gp-tree-name">{node.name}</span>
            {node.count > 0 && <span className="gp-tree-count">{node.count}</span>}
          </div>
          {hasChildren && expanded && renderTree(node.children, depth + 1)}
        </Fragment>
      );
    });

  return (
    <Modal
      className="gallery-picker-modal"
      title={t('从素材中心选择')}
      visible={visible}
      width={1080}
      centered
      bodyStyle={{ padding: 0 }}
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
      <div className="gp-layout">
        <aside className="gp-sidebar">
          <div className="gp-sidebar-head">{t('图片分组')}</div>
          <div className="gp-tree">
            <div
              className={`gp-tree-node gp-tree-root${galleryId === 0 ? ' is-active' : ''}`}
              onClick={() => selectFolder(0)}
            >
              <span className="gp-tree-arrow is-leaf" />
              <span className="gp-tree-name">{t('全部图片')}</span>
            </div>
            {renderTree(tree)}
          </div>
        </aside>
        <section className="gp-main">
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
            <span className="gp-current-folder">{selectedFolderName}</span>
          </div>
          <div className="gp-grid-wrap">
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
                        onClick={() => toggleSelect(pic)}
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
          </div>
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
        </section>
      </div>
    </Modal>
  );
}
