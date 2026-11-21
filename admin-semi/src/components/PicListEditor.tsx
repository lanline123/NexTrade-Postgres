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
 * PicList 的 showImage=false）：卡片不渲染缩略图（序号 + 删除钮 + 标题 +
 * 链接摘要），编辑弹窗只剩标题 / 描述 / 链接字段（无选图区、无商品选图
 * 入口），添加的空条目形状与原版 PicList 无图添加逐字一致
 *（{ picTitle, picDesc, picLink: { link, title } }，不带任何图片键）。
 * 缺省 true，既有调用方（PC 编辑器 + H5 全部图片列表）行为与落盘逐项不变；
 * 首个使用方为 H5 首焦轮播的导航文字列表（banner navList）。
 * showColor 逐项渐变色（2026-11-21 补形态，对齐原版 PicList 的 showColor）：
 * 原版 PicList 在 showColor 时每条目带「背景色」A/B 选色器且添加时同步写
 * gradientColorA/B 两键；此前我方无此形态——小图滑动等逐项消费渐变色的
 * 模块里，用户新加的条目永无渐变色、渲染端又无条件消费该字段，结果卡片
 * 无背景「没样式」且无处可改。现以可选 prop 补齐：弹窗内出背景色字段、
 * 新条目按 colorPalette 播种；缺省 false，全部既有调用方零变化。首个使用
 * 方为 H5 小图滑动（slidingImage）。
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
 * 卡片交互（2026-11-21 用户口径）：排序改长按拖动——左键按住卡片 250ms
 * 进入拖动态、移到目标卡松手落位（未达阈值的普通点击仍打开编辑弹窗，
 * 不再用拖拽柄）；卡片右上角改删除 X（与 ProductPickGrid 的
 * .dec-pick__remove 同形态：贴角 20px 深色半透明、悬停变红），点击走
 * removeAt 与弹窗内「删除」同一回写路径，弹窗删除入口保留。
 * 卡片缩略图口径（2026-11-21 对齐原版）：原版 PicList.vue 卡片图用
 * picThumb（:src="imageFormat(element.picThumb)"），此前我方 picUrl 优先——
 * 首焦轮播默认图 picUrl 为 webp 且在我方 OSS 上 Content-Type 为
 * application/octet-stream，严格引擎不渲染、卡片图裂；改 picThumb 优先、
 * picUrl 兜底，纯显示层、数据零变化。编辑弹窗内的图仍以 picUrl 为准
 * （编辑对象本体）。
 * 条目级标题颜色（2026-11-21，我方新增，用户指令）：可选 prop
 * showTitleColor（缺省 false）透传 PicItemEditModal 的同名开关，开启后
 * 条目可在编辑弹窗里单独设标题颜色（PicItem.titleColor，缺失 = 跟随
 * 模块级标题颜色）；卡片标题在字段在场时同步以该色呈现。首个启用方
 * 为商品提示 shopTips，既有全部调用点行为与落盘逐字不变。
 * 通用组件登记：~/workspace/shared-components.md 第 1 条。
 */
import { useRef, useState } from 'react';
import { Toast } from '@douyinfe/semi-ui';
import { IconClose, IconPlus } from '@douyinfe/semi-icons';
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
  /** 逐项渐变色 A/B（原版 PicList showColor 字段，小图滑动等模块逐项消费） */
  gradientColorA?: string;
  gradientColorB?: string;
  /** 条目级标题颜色（我方新增 2026-11-21，用户指令；缺失/空串 = 不设条目色、消费方回退模块级标题颜色，商品提示 shopTips 首个消费方） */
  titleColor?: string;
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
  showColor = false,
  colorPalette,
  showTitleColor = false,
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
  /**
   * true = 逐项渐变色模式（对齐原版 PicList 的 showColor，如小图滑动）：
   * 编辑弹窗内多「背景色」A/B 两色字段，新条目按 colorPalette 播种
   * gradientColorA/B（与模块默认条目同形状）；缺省 false，既有调用方零变化。
   */
  showColor?: boolean;
  /** showColor 时新条目的渐变色播种调色板（[A, B] 对，按条目下标循环取） */
  colorPalette?: Array<[string, string]>;
  /** true 时编辑弹窗多一条目级「标题颜色」行（写 item.titleColor，我方新增）；缺省 false，既有调用点零变化 */
  showTitleColor?: boolean;
  max?: number;
  onPickManyProducts?: (items: PickedImage[]) => void;
}) {
  const { t } = useI18n();
  const list = Array.isArray(value) ? value : [];
  /** showColor 播种：按条目下标在调色板内循环取一对渐变色（无调色板时为空对，键仍落盘） */
  const palettePair = (index: number): [string, string] =>
    colorPalette && colorPalette.length ? colorPalette[index % colorPalette.length] : ['', ''];
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const gridRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<number | null>(null);
  const overRef = useRef<number | null>(null);
  const suppressClickRef = useRef(false);
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
    dragRef.current = null;
    overRef.current = null;
    setDragIndex(null);
    setOverIndex(null);
  };
  /** 长按拖动排序（用户 2026-11-21 口径，替代旧拖拽柄 HTML5 DnD）：左键按住
   *  卡片 250ms 进入拖动态，指针移到目标卡松手即落位回写（reorder，数据形状
   *  不变）；未达阈值的普通点击仍走点卡打开编辑弹窗——拖完松手后的那次
   *  click 由 suppressClickRef 吞掉（只吞真实拖放产生的那一次，同画布
   *  palette-click 的点击抑制口径）。等待期内指针移动超过 8px 视为放弃
   *  长按（划选/滚动意图），不进拖动态。手势监听挂 window、松手即拆，
   *  闭包内 list/reorder 取按下那一渲染的值（手势期间列表不变）。 */
  const LONG_PRESS_MS = 250;
  const PRESS_MOVE_TOLERANCE = 8;
  const beginPress = (event: React.PointerEvent, index: number) => {
    if (event.pointerType === 'mouse' && event.button !== 0) return;
    if ((event.target as HTMLElement).closest('.dec-pic-grid__remove')) return;
    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;
    const cleanup = () => {
      window.clearTimeout(timer);
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging) {
        if (Math.hypot(e.clientX - startX, e.clientY - startY) > PRESS_MOVE_TOLERANCE) cleanup();
        return;
      }
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest('.dec-pic-grid__card') as HTMLElement | null;
      if (el && gridRef.current?.contains(el)) {
        const idx = Number(el.dataset.index);
        if (!Number.isNaN(idx) && idx !== overRef.current) {
          overRef.current = idx;
          setOverIndex(idx);
        }
      }
    };
    const onUp = () => {
      if (dragging) {
        suppressClickRef.current = true;
        if (overRef.current !== null) reorder(index, overRef.current);
      }
      cleanup();
      endDrag();
    };
    const timer = window.setTimeout(() => {
      dragging = true;
      dragRef.current = index;
      overRef.current = index;
      setDragIndex(index);
      setOverIndex(index);
    }, LONG_PRESS_MS);
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  };
  const removeAt = (index: number) => {
    onChange(list.filter((_, i) => i !== index));
    if (editIndex === index) setEditIndex(null);
    else if (editIndex !== null && editIndex > index) setEditIndex(editIndex - 1);
  };
  const addItem = () => {
    const emptyItem = showImage
      ? ({
          picUrl: '',
          picThumb: '',
          picName: '',
          picTitle: '',
          picLink: { link: '', name: '' },
          // showColor 模式：新条目出生即带渐变色（原版 PicList onAdd 在 showColor 时
          // 同步写 gradientColorA/B 两键），按其落位下标取调色板一对，与默认条目同形状
          ...(showColor
            ? { gradientColorA: palettePair(list.length)[0], gradientColorB: palettePair(list.length)[1] }
            : {}),
        } as PicItem)
      : // 无图模式空条目：与原版 PicList showImage=false 的添加形状逐字一致
        //（只有文字列表的实际字段 picTitle/picDesc/picLink，不带图片键）
        ({ picTitle: '', picDesc: '', picLink: { link: '', title: '' } } as unknown as PicItem);
    const next = [...list, emptyItem];
    onChange(next);
    setEditIndex(next.length - 1);
  };
  const renderIndexBadge = (index: number) => <span className="dec-pic-grid__index">{index + 1}</span>;
  const renderRemove = (index: number) => (
    <button
      type="button"
      className="dec-pic-grid__remove"
      aria-label={t('删除')}
      title={t('删除')}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        removeAt(index);
      }}
    >
      <IconClose />
    </button>
  );
  return (
    <div
      ref={gridRef}
      className={`${showImage ? 'dec-pic-grid' : 'dec-pic-grid dec-pic-grid--imageless'}${dragIndex !== null ? ' is-sorting' : ''}`}
    >
      {list.map((item, index) => {
        // 卡片缩略图：原版 PicList 口径 picThumb 优先（见文件头注），picUrl 兜底
        const url = String(item.picThumb || item.picUrl || '');
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
            data-index={index}
            role="button"
            tabIndex={0}
            onPointerDown={(event) => beginPress(event, index)}
            onClick={() => {
              if (suppressClickRef.current) {
                suppressClickRef.current = false;
                return;
              }
              setEditIndex(index);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') setEditIndex(index);
            }}
          >
            {showImage ? (
              <div className="dec-pic-grid__thumb">
                {url ? <img src={url} alt="" draggable={false} /> : <div className="dec-pic-grid__empty"><IconPlus /></div>}
                {renderIndexBadge(index)}
                {renderRemove(index)}
              </div>
            ) : (
              // 无图模式：序号与删除钮直接挂卡片（绝对定位口径与缩略图内一致）
              <>
                {renderIndexBadge(index)}
                {renderRemove(index)}
              </>
            )}
            <div className="dec-pic-grid__meta">
              {/* 条目级标题颜色在场时卡片标题同步以该色呈现（设置列表所见即所得）；无字段零变化 */}
              <div className="dec-pic-grid__title" style={item.titleColor ? { color: item.titleColor } : undefined}>{item.picTitle || t('未命名')}</div>
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
        showColor={showColor}
        colorFallback={editIndex !== null ? palettePair(editIndex) : undefined}
        showTitleColor={showTitleColor}
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
          if (showColor) {
            // 本次落位的新条目（含被首件回填的当前卡）按最终下标补渐变色，
            // 只补缺失、不覆盖既有值，与默认条目同形状
            const from = hasPic ? editIndex + 1 : editIndex;
            const to = Math.min(next.length - 1, from + usable.length - 1);
            for (let i = from; i <= to; i += 1) {
              if (next[i] && !String(next[i].gradientColorA || '')) {
                const pair = palettePair(i);
                next[i] = { ...next[i], gradientColorA: pair[0], gradientColorB: pair[1] };
              }
            }
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
