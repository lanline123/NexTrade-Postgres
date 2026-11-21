/**
 * 图片列表编辑器（PicListEditor）——PC / H5 装修编辑器共享的图片列表编辑组件。
 *
 * 2026-11-21 自 PcDecorateEditor 页内本地函数抽取升格为共享件（用户定版：
 * PC 编辑器与 H5 图片魔方调同一共享件，不再各写一套）：卡片网格（拖拽排序）
 * + 点卡打开「编辑图片」弹窗——弹窗本体已于同日续抽为独立共享件「图片编辑
 * 弹窗」PicItemEditModal（登记册第 16 条，可供其他场景独立调用），本组件
 * 只负责把当前条目与回写口喂给它；弹窗内工具栏「从商品选择图片」经其
 * onRequestProductPick 回到本组件唤起共享 ProductImagePickerModal（批量
 * 落位口径在本组件，见 onPickManyProducts 说明），素材中心选图走弹窗内
 * ImageUploader（内含 GalleryPickerModal）；标题 / 描述 / 链接在弹窗内编辑。
 * 编辑弹窗版式（2026-11-21 按用户指示调整，PC / H5 同形）：选图框在左、
 * 标题 / 链接字段纵列在右（.dec-pic-modal__body 横向布局）；抽取前 PC 现状
 * 为选图框居中在上、字段纵向堆叠在下。
 * onPickManyProducts 为可选的商品批量落位委托：传入时 onPickMany 的列表
 * 落位由调用方接管（H5 图片魔方：定长槽位自当前位顺延填充，商品链接的
 * H5 LinkType 口径在调用点适配）；不传时走内置口径（当前卡填首件、余者
 * 在其后插入新卡，受 max 容量约束，链接为 PC 的 /product/{id} 形状）。
 * PC 编辑器不传此 prop，行为与抽取前逐项一致。
 * showImage=false 为可选的「无图条目」模式（2026-11-21 补形态，对齐原版
 * PicList 的 showImage=false）：卡片不渲染缩略图（序号 + 拖拽柄 + 标题 +
 * 链接摘要），编辑弹窗只剩标题 / 描述 / 链接字段（无选图区、无商品选图
 * 入口），添加的空条目形状与原版 PicList 无图添加逐字一致
 *（{ picTitle, picDesc, picLink: { link, title } }，不带任何图片键）。
 * 缺省 true，既有调用方（PC 编辑器 + H5 全部图片列表）行为与落盘逐项不变；
 * 首个使用方为 H5 首焦轮播的导航文字列表（banner navList）。
 * 样式：pages/decorate-editor.css 的 .dec-pic-grid* / .dec-pic-modal*
 * （PC 与 H5 编辑器入口均已引入该样式表）。
 * 编辑弹窗现代化（2026-11-21，用户在 10023 页提出）：弹窗壳改直角扁平
 * （.dec-pic-modal__dialog 作用域：描边 + 柔和投影、头/脚描边分隔）、
 * 工具栏改浅底描边操作条、预览区改描边面板、字段标签加重；功能与数据
 * 契约零变化（选图/清除/素材中心更换/标题/描述/链接/删除/拖拽排序全保留）。
 * 弹窗独立化续批（同日，用户在分类推荐 A1「基本设置」提出）：弹窗抽为
 * 独立组件 PicItemEditModal 并改垂直居中（centered，此前贴顶呈现）、
 * 内容超高时定高内部滚动；本组件调用口径与行为逐字不变。
 * 圆角口径核查（同日）：.dec-pic-grid__card / .dec-pic-modal__preview /
 * uploader 条目在 main 现行 CSS 中已为 border-radius: 0（直角扁平，与用户
 * 在现网旧版 admin 上看到的 14px 圆角不是同一版本），本批保持 0 不回加。
 * 通用组件登记：~/workspace/shared-components.md 第 1 条。
 */
import { useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import { IconHandle, IconPlus } from '@douyinfe/semi-icons';
import { useI18n } from '../i18n';
import ProductImagePickerModal, { type PickedImage } from './ProductImagePicker';
import PicItemEditModal from './PicItemEditModal';

/** 图片条目（与 decoratePc/contract 的 PicItem 逐字段同形，抽取时对齐） */
export interface PicItem {
  picId?: string;
  picUrl: string;
  picThumb?: string;
  picName?: string;
  picTitle?: string;
  picDesc?: string;
  /** 分类导航3 卡片级图片背景色（原版 PcCategoryNav3Item） */
  imageBackgroundColor?: string;
  picLink?: { link?: string; name?: string; title?: string };
}

export function PicListEditor({
  value,
  onChange,
  showDesc = false,
  addText,
  showLink = true,
  showText = true,
  showImage = true,
  max,
  onPickManyProducts,
}: {
  value: PicItem[];
  onChange: (next: PicItem[]) => void;
  showDesc?: boolean;
  addText?: string;
  showLink?: boolean;
  showText?: boolean;
  /** false = 无图条目模式（纯文字 + 链接列表，如导航文字）；缺省 true */
  showImage?: boolean;
  max?: number;
  onPickManyProducts?: (items: PickedImage[]) => void;
}) {
  const { t } = useI18n();
  const list = Array.isArray(value) ? value : [];
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const [editIndex, setEditIndex] = useState<number | null>(null);
  const [productPickerOpen, setProductPickerOpen] = useState(false);
  const updateItem = (index: number, patch: Record<string, any>) => {
    onChange(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
  };
  const reorder = (from: number, to: number) => {
    if (from === to) return;
    const next = [...list];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    onChange(next);
  };
  const endDrag = () => {
    setDragIndex(null);
    setOverIndex(null);
  };
  const removeAt = (index: number) => {
    onChange(list.filter((_, i) => i !== index));
    if (editIndex === index) setEditIndex(null);
    else if (editIndex !== null && editIndex > index) setEditIndex(editIndex - 1);
  };
  const addItem = () => {
    const emptyItem = showImage
      ? ({ picUrl: '', picThumb: '', picName: '', picTitle: '', picLink: { link: '', name: '' } } as PicItem)
      : // 无图模式空条目：与原版 PicList showImage=false 的添加形状逐字一致
        //（只有文字列表的实际字段 picTitle/picDesc/picLink，不带图片键）
        ({ picTitle: '', picDesc: '', picLink: { link: '', title: '' } } as unknown as PicItem);
    const next = [...list, emptyItem];
    onChange(next);
    setEditIndex(next.length - 1);
  };
  const renderIndexBadge = (index: number) => <span className="dec-pic-grid__index">{index + 1}</span>;
  const renderDragHandle = (index: number) => (
    <span
      className="dec-pic-grid__handle"
      draggable
      title={t('拖拽排序')}
      onClick={(event) => event.stopPropagation()}
      onDragStart={(event) => {
        event.stopPropagation();
        setDragIndex(index);
        event.dataTransfer.effectAllowed = 'move';
        event.dataTransfer.setData('text/plain', String(index));
      }}
      onDragEnd={endDrag}
    >
      <IconHandle />
    </span>
  );
  return (
    <div className={showImage ? 'dec-pic-grid' : 'dec-pic-grid dec-pic-grid--imageless'}>
      {list.map((item, index) => {
        const url = String(item.picUrl || item.picThumb || '');
        // 无图卡片的次行摘要：链接目标（link 或 H5 LinkType 的 path），无描述时展示
        const linkText = showImage
          ? ''
          : String(item.picLink?.link || (item.picLink as Record<string, any> | undefined)?.path || '');
        return (
          <div
            className={`dec-pic-grid__card${showImage ? '' : ' dec-pic-grid__card--imageless'}${
              dragIndex === index ? ' is-dragging' : ''
            }${overIndex === index && dragIndex !== null && dragIndex !== index ? ' is-drop-target' : ''}`}
            key={String(item.picId || index)}
            role="button"
            tabIndex={0}
            onClick={() => setEditIndex(index)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') setEditIndex(index);
            }}
            onDragOver={(event) => {
              if (dragIndex === null) return;
              event.preventDefault();
              if (overIndex !== index) setOverIndex(index);
            }}
            onDrop={(event) => {
              event.preventDefault();
              if (dragIndex !== null) reorder(dragIndex, index);
              endDrag();
            }}
          >
            {showImage ? (
              <div className="dec-pic-grid__thumb">
                {url ? <img src={url} alt="" /> : <div className="dec-pic-grid__empty"><IconPlus /></div>}
                {renderIndexBadge(index)}
                {renderDragHandle(index)}
              </div>
            ) : (
              // 无图模式：序号与拖拽柄直接挂卡片（绝对定位口径与缩略图内一致）
              <>
                {renderIndexBadge(index)}
                {renderDragHandle(index)}
              </>
            )}
            <div className="dec-pic-grid__meta">
              <div className="dec-pic-grid__title">{item.picTitle || t('未命名')}</div>
              {showDesc && item.picDesc ? (
                <div className="dec-pic-grid__desc">{item.picDesc}</div>
              ) : linkText ? (
                <div className="dec-pic-grid__desc">{linkText}</div>
              ) : null}
            </div>
          </div>
        );
      })}
      {(!max || list.length < max) && (
        <button type="button" className="dec-pic-grid__add" onClick={addItem}>
          <span className="dec-pic-grid__add-icon"><IconPlus /></span>
          <span>{t(addText || '添加图片')}</span>
        </button>
      )}
      <PicItemEditModal
        visible={editIndex !== null}
        item={editIndex !== null ? list[editIndex] : undefined}
        showImage={showImage}
        showText={showText}
        showDesc={showDesc}
        showLink={showLink}
        onPatch={(patch) => {
          if (editIndex !== null) updateItem(editIndex, patch);
        }}
        onRemove={() => {
          if (editIndex !== null) removeAt(editIndex);
        }}
        onClose={() => setEditIndex(null)}
        onRequestProductPick={showImage ? () => setProductPickerOpen(true) : undefined}
      />
      {showImage ? (
      <ProductImagePickerModal
        visible={productPickerOpen}
        onCancel={() => setProductPickerOpen(false)}
        onPick={(url) => {
          if (editIndex !== null) updateItem(editIndex, { picUrl: url, picThumb: url });
          setProductPickerOpen(false);
        }}
        onPickMany={(items) => {
          if (onPickManyProducts) {
            if (items.length) onPickManyProducts(items);
            setProductPickerOpen(false);
            return;
          }
          if (editIndex === null || !items.length) return;
          const currentItem = list[editIndex];
          const hasPic = !!String(currentItem?.picUrl || currentItem?.picThumb || '');
          const capacity = max ? Math.max(0, max - list.length) : Number.POSITIVE_INFINITY;
          // 当前卡片为空时第一张直接填它，不占用新增名额
          const allowance = hasPic ? capacity : capacity + 1;
          const usable = items.slice(0, allowance);
          if (items.length > usable.length) Toast.warning(t('最多只能添加 {max} 张图片', { max: max ?? 0 }));
          if (!usable.length) {
            setProductPickerOpen(false);
            return;
          }
          // 商品名做标题，链接指向 PC 前台商品详情页 /product/{id}
          const makeItem = (p: PickedImage): PicItem => ({
            picUrl: p.url,
            picThumb: p.url,
            picName: p.productName,
            picTitle: p.productName,
            picDesc: '',
            picLink: {
              link: `/product/${p.productId}`,
              name: p.productName,
              label: t('商品详情页'),
            },
          } as PicItem);
          const next = [...list];
          if (!hasPic) {
            next[editIndex] = { ...currentItem, ...makeItem(usable[0]) };
            next.splice(editIndex + 1, 0, ...usable.slice(1).map(makeItem));
          } else {
            next.splice(editIndex + 1, 0, ...usable.map(makeItem));
          }
          onChange(next);
          Toast.success(t('已带入 {count} 张商品图片（含标题和链接）', { count: usable.length }));
          setProductPickerOpen(false);
        }}
      />
      ) : null}
    </div>
  );
}

export default PicListEditor;
