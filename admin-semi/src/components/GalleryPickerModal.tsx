import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import { Button, Empty, Input, Modal, Pagination, Toast } from '@douyinfe/semi-ui';
import { IconClose, IconGallery, IconPlayCircle, IconSearch, IconVideo } from '@douyinfe/semi-icons';
import { apiGet, apiPost, apiUpload, isDemo } from '../api';
import { useI18n } from '../i18n';
import './gallery-picker.css';

interface MediaItem {
  id?: number;
  galleryId?: number;
  picName?: string;
  videoName?: string;
  picUrl?: string;
  picThumb?: string;
  videoUrl?: string;
  videoCover?: string;
  videoFirstFrame?: string;
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
  mediaType?: 'image' | 'video';
  multiple?: boolean;
  limit?: number;
  onCancel: () => void;
  onConfirm: (urls: string[]) => void;
}

const PAGE_SIZE = 30;
const SKELETON_COUNT = 15;

const normalizeTree = (nodes: RawGalleryNode[]): GalleryTreeNode[] =>
  (nodes || []).map((node) => ({
    id: Number(node.galleryId ?? node.id ?? 0),
    name: node.galleryName || node.name || '',
    count: Number(node.picCount ?? 0),
    children: normalizeTree(node.children || []),
  }));

const filterTree = (nodes: GalleryTreeNode[], keyword: string): GalleryTreeNode[] => {
  const text = keyword.trim().toLowerCase();
  if (!text) return nodes;
  return nodes.reduce<GalleryTreeNode[]>((acc, node) => {
    const children = filterTree(node.children, keyword);
    if (node.name.toLowerCase().includes(text) || children.length) {
      acc.push({ ...node, children });
    }
    return acc;
  }, []);
};

const videoNameFromFile = (file: File) => {
  const base = file.name.replace(/\.[^.]+$/, '');
  return (base || file.name).slice(0, 20);
};

const readVideoDuration = (file: File) =>
  new Promise<string>((resolve) => {
    const url = URL.createObjectURL(file);
    const video = document.createElement('video');
    const done = (value: string) => {
      URL.revokeObjectURL(url);
      video.remove();
      resolve(value);
    };
    video.preload = 'metadata';
    video.onloadedmetadata = () => done(String(Math.round(video.duration || 0)));
    video.onerror = () => done('');
    video.src = url;
  });

export default function GalleryPickerModal({
  visible,
  mediaType = 'image',
  multiple = true,
  limit,
  onCancel,
  onConfirm,
}: GalleryPickerModalProps) {
  const { t } = useI18n();
  const isVideo = mediaType === 'video';
  const [keyword, setKeyword] = useState('');
  const [groupKeyword, setGroupKeyword] = useState('');
  const [galleryId, setGalleryId] = useState(0);
  const [expandedIds, setExpandedIds] = useState<number[]>([]);
  const [page, setPage] = useState(1);
  const [items, setItems] = useState<MediaItem[]>([]);
  const [total, setTotal] = useState(0);
  const [tree, setTree] = useState<GalleryTreeNode[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const fileRef = useRef<HTMLInputElement>(null);

  const urlOf = (item: MediaItem) => (isVideo ? item.videoUrl || '' : item.picUrl || item.picThumb || '');
  const thumbOf = (item: MediaItem) =>
    isVideo ? item.videoCover || item.videoFirstFrame || '' : item.picThumb || item.picUrl || '';
  const nameOf = (item: MediaItem) => (isVideo ? item.videoName || '' : item.picName || '');

  useEffect(() => {
    if (!visible) return;
    setSelected([]);
    setKeyword('');
    setGroupKeyword('');
    setGalleryId(0);
    setExpandedIds([]);
    setPage(1);
    if (isVideo) {
      apiGet<any>('/setting/galleryVideo/getAllCategory')
        .then((result) => setTree(normalizeTree(Array.isArray(result) ? result : result?.records || [])))
        .catch(() => undefined);
    } else {
      apiGet<{ records?: RawGalleryNode[] }>('/setting/gallery/tree', { type: 'image' })
        .then((result) => setTree(normalizeTree(result?.records || [])))
        .catch(() => undefined);
    }
  }, [visible, isVideo]);

  useEffect(() => {
    if (!visible || isDemo()) return;
    let alive = true;
    setLoading(true);
    const endpoint = isVideo ? '/setting/galleryVideoInfo/list' : '/setting/galleryPic/list';
    const params = isVideo
      ? {
          page,
          size: PAGE_SIZE,
          id: galleryId || undefined,
          keyword: keyword.trim() || undefined,
          sortOrder: 'desc',
        }
      : {
          page,
          size: PAGE_SIZE,
          galleryId: galleryId || undefined,
          keyword: keyword.trim() || undefined,
          sortOrder: 'desc',
        };
    apiGet<any>(endpoint, params)
      .then((result) => {
        if (!alive) return;
        const pageData = isVideo ? result?.galleryVideoInfoPage : result?.galleryPicPage;
        const records: MediaItem[] = pageData?.records || result?.records || result?.list || [];
        setItems(records.filter((item) => urlOf(item)));
        setTotal(Number(pageData?.total ?? result?.total ?? records.length));
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
  }, [visible, page, galleryId, reloadKey, isVideo, t]);

  const visibleTree = useMemo(() => filterTree(tree, groupKeyword), [tree, groupKeyword]);

  const selectedFolderName = useMemo(() => {
    if (galleryId === 0) return isVideo ? t('全部视频') : t('全部图片');
    const find = (nodes: GalleryTreeNode[]): string | undefined => {
      for (const node of nodes) {
        if (node.id === galleryId) return node.name;
        const hit = find(node.children);
        if (hit) return hit;
      }
      return undefined;
    };
    return find(tree) || (isVideo ? t('全部视频') : t('全部图片'));
  }, [galleryId, tree, isVideo, t]);

  const toggleExpand = (id: number) => {
    setExpandedIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );
  };

  const selectFolder = (id: number) => {
    setGalleryId(id);
    setPage(1);
  };

  const toggleSelect = (item: MediaItem) => {
    const url = urlOf(item);
    if (!url) return;
    setSelected((current) => {
      if (current.includes(url)) return current.filter((entry) => entry !== url);
      if (!multiple) return [url];
      if (limit !== undefined && current.length >= limit) {
        Toast.warning({ content: t('最多选择 {count} 张', { count: limit }) });
        return current;
      }
      return [...current, url];
    });
  };

  const pickAndConfirm = (item: MediaItem) => {
    const url = urlOf(item);
    if (!url) return;
    if (!multiple) {
      onConfirm([url]);
      return;
    }
    const next = selected.includes(url) ? selected : [...selected, url];
    onConfirm(limit !== undefined ? next.slice(0, limit) : next);
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
        if (isVideo) {
          const data = await apiUpload<Record<string, any>>('/setting/galleryVideoInfo/uploadVideo', file);
          const duration = await readVideoDuration(file);
          await apiPost('/setting/galleryVideoInfo/create', {
            ...data,
            videoName: videoNameFromFile(file),
            duration,
            size: `${(file.size / 1024 / 1024).toFixed(2)}MB`,
            videoCover: '',
            galleryId: [galleryId || 0],
            source: 'upload',
          });
          if (data?.videoUrl) urls.push(String(data.videoUrl));
        } else {
          const result = await apiUpload<{ picUrl?: string }>(
            '/setting/galleryPic/uploadImg?galleryId=0',
            file,
          );
          if (result?.picUrl) urls.push(result.picUrl);
        }
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
        Toast.success({ content: isVideo ? t('视频上传成功') : t('图片上传成功') });
      }
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('上传失败') });
    } finally {
      setUploading(false);
      if (fileRef.current) fileRef.current.value = '';
    }
  };

  const renderTree = (nodes: GalleryTreeNode[], depth = 0) =>
    nodes.map((node) => {
      const hasChildren = node.children.length > 0;
      const expanded = groupKeyword.trim() !== '' || expandedIds.includes(node.id);
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
                if (hasChildren && !groupKeyword.trim()) toggleExpand(node.id);
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
      title={isVideo ? t('从素材中心选择视频') : t('从素材中心选择图片')}
      visible={visible}
      width="min(1280px, 94vw)"
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
          <div className="gp-sidebar-head">
            <span className="gp-sidebar-title">{isVideo ? t('视频分组') : t('图片分组')}</span>
            <Input
              className="gp-group-search"
              size="small"
              prefix={<IconSearch />}
              value={groupKeyword}
              placeholder={t('搜索分组')}
              showClear
              onChange={setGroupKeyword}
            />
          </div>
          <div className="gp-tree">
            <div
              className={`gp-tree-node gp-tree-root${galleryId === 0 ? ' is-active' : ''}`}
              onClick={() => selectFolder(0)}
            >
              <span className="gp-tree-arrow is-leaf" />
              <span className="gp-tree-name">{isVideo ? t('全部视频') : t('全部图片')}</span>
            </div>
            {renderTree(visibleTree)}
          </div>
        </aside>
        <section className="gp-main">
          <div className="gp-toolbar">
            <Button
              type="primary"
              theme="solid"
              icon={isVideo ? <IconVideo /> : <IconGallery />}
              loading={uploading}
              onClick={() => fileRef.current?.click()}
            >
              {isVideo ? t('上传视频') : t('上传图片')}
            </Button>
            <input
              ref={fileRef}
              type="file"
              accept={isVideo ? 'video/mp4' : 'image/*'}
              multiple
              hidden
              onChange={(event) => void upload(event.target.files)}
            />
            <Input
              className="gp-search"
              prefix={<IconSearch />}
              value={keyword}
              placeholder={isVideo ? t('视频名称') : t('图片名称')}
              showClear
              onChange={setKeyword}
              onEnterPress={() => {
                setPage(1);
                setReloadKey((key) => key + 1);
              }}
            />
            <span className="gp-folder-chip">{selectedFolderName}</span>
            <span className="gp-total">{total}</span>
          </div>
          <div className="gp-grid-wrap">
            {loading && !items.length ? (
              <div className="gp-grid">
                {Array.from({ length: SKELETON_COUNT }).map((_, index) => (
                  <div className="gp-skeleton" key={index} />
                ))}
              </div>
            ) : items.length ? (
              <div className="gp-grid">
                {items.map((item, index) => {
                  const url = urlOf(item);
                  const thumb = thumbOf(item);
                  const name = nameOf(item);
                  const isSelected = selected.includes(url);
                  return (
                    <div
                      key={`${item.id ?? item.galleryId ?? index}-${url}`}
                      className={`gp-item${isVideo ? ' is-video' : ''}${isSelected ? ' is-selected' : ''}`}
                      title={name}
                      onClick={() => toggleSelect(item)}
                      onDoubleClick={() => pickAndConfirm(item)}
                    >
                      {thumb ? (
                        <img src={thumb} alt={name} loading="lazy" />
                      ) : (
                        <div className="gp-video-placeholder">
                          <IconVideo size="extra-large" />
                        </div>
                      )}
                      {isVideo && (
                        <span className="gp-play">
                          <IconPlayCircle size="large" />
                        </span>
                      )}
                      <span className="gp-item-check">{isSelected ? '✓' : ''}</span>
                      {name && <span className="gp-name">{name}</span>}
                    </div>
                  );
                })}
              </div>
            ) : (
              !loading && <Empty description={t('暂无素材')} />
            )}
          </div>
          {selected.length > 0 && (
            <div className="gp-tray">
              <div className="gp-tray-list">
                {selected.map((url) => {
                  const picked = items.find((item) => urlOf(item) === url);
                  const thumb = picked ? thumbOf(picked) : '';
                  return (
                    <div className="gp-tray-item" key={url}>
                      {thumb ? <img src={thumb} alt="" /> : <span className="gp-tray-video"><IconVideo /></span>}
                      <button
                        type="button"
                        aria-label={t('移除')}
                        onClick={() => setSelected((current) => current.filter((item) => item !== url))}
                      >
                        <IconClose size="small" />
                      </button>
                    </div>
                  );
                })}
              </div>
              <button type="button" className="gp-tray-clear" onClick={() => setSelected([])}>
                {t('清空')}
              </button>
            </div>
          )}
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
