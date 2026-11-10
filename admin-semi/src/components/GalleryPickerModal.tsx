import { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import type { DragEvent as ReactDragEvent } from 'react';
import {
  Button,
  Dropdown,
  Empty,
  Input,
  Modal,
  Pagination,
  Radio,
  RadioGroup,
  Toast,
} from '@douyinfe/semi-ui';
import { IconClose, IconGallery, IconPlayCircle, IconSearch, IconVideo } from '@douyinfe/semi-icons';
import { apiGet, apiPost, apiUpload, isDemo } from '../api';
import { useI18n } from '../i18n';
import './gallery-picker.css';

interface MediaItem {
  id?: number;
  picId?: number;
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
  parentId: number;
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
    parentId: Number(node.parentId ?? 0),
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
  const [dragId, setDragId] = useState<number | null>(null);
  const [dropFolderId, setDropFolderId] = useState<number | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [formMode, setFormMode] = useState<'create' | 'renameFolder' | 'renameItem'>('create');
  const [formParentId, setFormParentId] = useState(0);
  const [formFolder, setFormFolder] = useState<GalleryTreeNode | null>(null);
  const [formTarget, setFormTarget] = useState<MediaItem | null>(null);
  const [formName, setFormName] = useState('');
  const [formSaving, setFormSaving] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const urlOf = (item: MediaItem) => (isVideo ? item.videoUrl || '' : item.picUrl || item.picThumb || '');
  const thumbOf = (item: MediaItem) =>
    isVideo ? item.videoCover || item.videoFirstFrame || '' : item.picThumb || item.picUrl || '';
  const nameOf = (item: MediaItem) => (isVideo ? item.videoName || '' : item.picName || '');
  const idOf = (item: MediaItem) => Number(isVideo ? item.id ?? 0 : item.picId ?? 0);

  const loadTree = async () => {
    try {
      if (isVideo) {
        const result = await apiGet<any>('/setting/galleryVideo/getAllCategory');
        setTree(normalizeTree(Array.isArray(result) ? result : result?.records || []));
      } else {
        const result = await apiGet<{ records?: RawGalleryNode[] }>('/setting/gallery/tree', {
          type: 'image',
        });
        setTree(normalizeTree(result?.records || []));
      }
    } catch {
      // 分组加载失败不阻塞素材列表
    }
  };

  useEffect(() => {
    if (!visible) return;
    setSelected([]);
    setKeyword('');
    setGroupKeyword('');
    setGalleryId(0);
    setExpandedIds([]);
    setPage(1);
    void loadTree();
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  const moveItemToFolder = async (folderId: number) => {
    const itemId = dragId;
    setDragId(null);
    setDropFolderId(null);
    if (!itemId || folderId === galleryId) return;
    try {
      await apiPost('/setting/galleryPic/transfer', {
        ids: [itemId],
        targetGalleryId: folderId,
        type: isVideo ? 'video' : 'image',
      });
      Toast.success({ content: t('移动成功') });
      setReloadKey((key) => key + 1);
      void loadTree();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('移动失败') });
    }
  };

  const openCreateFolder = (parentId: number) => {
    setFormMode('create');
    setFormParentId(parentId);
    setFormFolder(null);
    setFormTarget(null);
    setFormName('');
    setFormOpen(true);
  };

  const openRenameFolder = (node: GalleryTreeNode) => {
    setFormMode('renameFolder');
    setFormFolder(node);
    setFormTarget(null);
    setFormName(node.name);
    setFormOpen(true);
  };

  const openRenameItem = (item: MediaItem) => {
    setFormMode('renameItem');
    setFormTarget(item);
    setFormFolder(null);
    setFormName(nameOf(item));
    setFormOpen(true);
  };

  const submitForm = async () => {
    const name = formName.trim();
    if (!name) {
      Toast.warning({
        content:
          formMode === 'create' || formMode === 'renameFolder'
            ? t('请输入分组名称')
            : isVideo
              ? t('请输入视频名称')
              : t('请输入图片名称'),
      });
      return;
    }
    setFormSaving(true);
    try {
      if (formMode === 'create') {
        await apiPost(
          isVideo ? '/setting/galleryVideo/create' : '/setting/gallery/create',
          isVideo ? { parentId: formParentId, name } : { parentId: formParentId, galleryName: name },
        );
        setExpandedIds((current) =>
          current.includes(formParentId) ? current : [...current, formParentId],
        );
        void loadTree();
      } else if (formMode === 'renameFolder' && formFolder) {
        await apiPost(
          isVideo ? '/setting/galleryVideo/update' : '/setting/gallery/update',
          isVideo
            ? { id: formFolder.id, parentId: formFolder.parentId ?? 0, name }
            : {
                id: formFolder.id,
                parentId: formFolder.parentId ?? 0,
                galleryName: name,
              },
        );
        void loadTree();
      } else if (formTarget) {
        if (isVideo) {
          await apiPost('/setting/galleryVideoInfo/update', { ...formTarget, videoName: name });
        } else {
          await apiPost('/setting/galleryPic/updateField', {
            id: idOf(formTarget),
            field: 'picName',
            val: name,
          });
        }
      }
      Toast.success({ content: t('保存成功') });
      setFormOpen(false);
      setReloadKey((key) => key + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setFormSaving(false);
    }
  };

  const sortFolder = async (
    node: GalleryTreeNode,
    direction: 'up' | 'down',
    siblings: GalleryTreeNode[],
  ) => {
    const index = siblings.findIndex((item) => item.id === node.id);
    const targetIndex = direction === 'up' ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= siblings.length) return;
    const ids = siblings.map((item) => item.id);
    [ids[index], ids[targetIndex]] = [ids[targetIndex], ids[index]];
    try {
      await apiPost('/setting/gallery/sort', { ids, type: isVideo ? 2 : 1 });
      Toast.success({ content: t('排序成功') });
      void loadTree();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('排序失败') });
    }
  };

  const containsFolder = (nodes: GalleryTreeNode[], targetId: number): boolean =>
    nodes.some((node) => node.id === targetId || containsFolder(node.children, targetId));

  const deleteFolder = async (node: GalleryTreeNode, deleteMode: string) => {
    try {
      await apiPost(isVideo ? '/setting/galleryVideo/del' : '/setting/gallery/del', {
        id: node.id,
        deleteMode,
      });
      Toast.success({ content: t('删除成功') });
      if (galleryId === node.id || containsFolder(node.children, galleryId)) {
        setGalleryId(0);
        setPage(1);
      }
      void loadTree();
      setReloadKey((key) => key + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const openDeleteFolder = (node: GalleryTreeNode) => {
    const choice = { mode: 'only_group' };
    Modal.confirm({
      title: `${t('删除分组')}「${node.name}」`,
      okType: 'danger',
      content: (
        <div className="gp-delete-body">
          <RadioGroup
            defaultValue="only_group"
            onChange={(event) => {
              choice.mode = String(event.target.value);
            }}
          >
            <Radio value="only_group">
              <div className="gp-delete-option">
                <b>{t('仅删除分组(推荐)')}</b>
                <span>{t('仅删除分组，组内的素材会移出分组，仍可在全部列表中找到')}</span>
              </div>
            </Radio>
            <Radio value="group_and_files">
              <div className="gp-delete-option">
                <b>{t('删除分组及组内素材')}</b>
                <span>{t('分组及组内素材将会一并删除，删除不会对已使用图片的业务产生影响。')}</span>
              </div>
            </Radio>
          </RadioGroup>
        </div>
      ),
      onOk: () => deleteFolder(node, choice.mode),
    });
  };

  const confirmDeleteItem = (item: MediaItem) => {
    Modal.confirm({
      title: t('确认删除该{type}吗?', { type: isVideo ? t('视频') : t('图片') }),
      onOk: async () => {
        try {
          await apiPost(
            isVideo ? '/setting/galleryVideoInfo/del' : '/setting/galleryPic/del',
            { id: idOf(item) },
          );
          Toast.success({ content: t('删除成功') });
          setSelected((current) => current.filter((url) => url !== urlOf(item)));
          setReloadKey((key) => key + 1);
          void loadTree();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
        }
      },
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

  const folderDropProps = (folderId: number) => ({
    onDragOver: (event: ReactDragEvent) => {
      if (dragId === null) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = 'move';
      setDropFolderId(folderId);
    },
    onDragLeave: () => {
      setDropFolderId((current) => (current === folderId ? null : current));
    },
    onDrop: (event: ReactDragEvent) => {
      if (dragId === null) return;
      event.preventDefault();
      event.stopPropagation();
      void moveItemToFolder(folderId);
    },
  });

  const renderTree = (nodes: GalleryTreeNode[], depth = 0) =>
    nodes.map((node) => {
      const hasChildren = node.children.length > 0;
      const expanded = groupKeyword.trim() !== '' || expandedIds.includes(node.id);
      const active = galleryId === node.id;
      const row = (
        <div
          className={`gp-tree-node${active ? ' is-active' : ''}${dropFolderId === node.id ? ' is-drop' : ''}`}
          style={{ paddingLeft: 10 + depth * 16 }}
          onClick={() => selectFolder(node.id)}
          title={node.name}
          {...folderDropProps(node.id)}
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
          <span
            className="gp-tree-add"
            title={t('新建分组')}
            onClick={(event) => {
              event.stopPropagation();
              openCreateFolder(node.id);
            }}
          >
            +
          </span>
        </div>
      );
      return (
        <Fragment key={node.id}>
          <Dropdown
            trigger="contextMenu"
            position="bottomLeft"
            render={
              <Dropdown.Menu>
                <Dropdown.Item onClick={() => openCreateFolder(node.id)}>
                  {t('添加子分组')}
                </Dropdown.Item>
                <Dropdown.Item onClick={() => openRenameFolder(node)}>
                  {t('重命名')}
                </Dropdown.Item>
                <Dropdown.Item onClick={() => void sortFolder(node, 'up', nodes)}>
                  {t('上移')}
                </Dropdown.Item>
                <Dropdown.Item onClick={() => void sortFolder(node, 'down', nodes)}>
                  {t('下移')}
                </Dropdown.Item>
                <Dropdown.Item onClick={() => openDeleteFolder(node)}>
                  {t('删除')}
                </Dropdown.Item>
              </Dropdown.Menu>
            }
          >
            {row}
          </Dropdown>
          {hasChildren && expanded && renderTree(node.children, depth + 1)}
        </Fragment>
      );
    });

  return (
    <Modal
      className="gallery-picker-modal"
      title={isVideo ? t('从素材中心选择视频') : t('从素材中心选择图片')}
      visible={visible}
      width="min(1400px, 96vw)"
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
              className={`gp-tree-node gp-tree-root${galleryId === 0 ? ' is-active' : ''}${dropFolderId === 0 ? ' is-drop' : ''}`}
              onClick={() => selectFolder(0)}
              {...folderDropProps(0)}
            >
              <span className="gp-tree-arrow is-leaf" />
              <span className="gp-tree-name">{isVideo ? t('全部视频') : t('全部图片')}</span>
              <span
                className="gp-tree-add"
                title={t('新建分组')}
                onClick={(event) => {
                  event.stopPropagation();
                  openCreateFolder(0);
                }}
              >
                +
              </span>
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
                    <Dropdown
                      key={`${item.id ?? item.picId ?? index}-${url}`}
                      trigger="contextMenu"
                      position="bottomLeft"
                      render={
                        <Dropdown.Menu>
                          <Dropdown.Item onClick={() => openRenameItem(item)}>
                            {t('重命名')}
                          </Dropdown.Item>
                          <Dropdown.Item onClick={() => confirmDeleteItem(item)}>
                            {t('删除')}
                          </Dropdown.Item>
                        </Dropdown.Menu>
                      }
                    >
                      <div
                        className={`gp-item${isVideo ? ' is-video' : ''}${isSelected ? ' is-selected' : ''}${dragId === idOf(item) ? ' is-dragging' : ''}`}
                        title={name}
                        draggable
                        onClick={() => toggleSelect(item)}
                        onDoubleClick={() => pickAndConfirm(item)}
                        onDragStart={(event) => {
                          const id = idOf(item);
                          if (!id) return;
                          setDragId(id);
                          event.dataTransfer.effectAllowed = 'move';
                          event.dataTransfer.setData('text/plain', String(id));
                        }}
                        onDragEnd={() => {
                          setDragId(null);
                          setDropFolderId(null);
                        }}
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
                    </Dropdown>
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
      <Modal
        title={
          formMode === 'create'
            ? t('新建分组')
            : formMode === 'renameFolder'
              ? t('重命名分组')
              : t('重命名')
        }
        visible={formOpen}
        width={420}
        confirmLoading={formSaving}
        okText={t('确定')}
        onOk={submitForm}
        onCancel={() => setFormOpen(false)}
      >
        <div className="gp-form">
          <label>
            {formMode !== 'renameItem'
              ? t('分组名称')
              : isVideo
                ? t('视频名称')
                : t('图片名称')}
          </label>
          <Input
            value={formName}
            maxLength={40}
            placeholder={
              formMode !== 'renameItem'
                ? t('请输入分组名称')
                : isVideo
                  ? t('请输入视频名称')
                  : t('请输入图片名称')
            }
            onChange={setFormName}
            onEnterPress={submitForm}
          />
        </div>
      </Modal>
    </Modal>
  );
}
