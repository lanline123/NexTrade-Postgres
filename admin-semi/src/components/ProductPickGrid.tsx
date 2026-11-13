/**
 * 通用「已选商品」卡片网格 —— 装修编辑器选品控件的共用 UI。
 *
 * 刻意不绑定任何数据字段：调用方负责把 ids / items 读写到自己的数据里
 * （`products` 字段用 productIds + productItems，商品分组用 productIds + productList）。
 * 选品弹窗也由调用方管理，本组件只通过 onRequestPick 通知「该打开弹窗了」。
 *
 * 能力：缩略图卡片（图/名/价/序号/悬停移除）、拖拽排序、虚线添加卡、
 * 空态、已选计数与清空（二次确认）、历史数据缺展示缓存时按 ids 自动补全商品信息。
 */
import { useEffect, useRef, useState } from 'react';
import { IconPlus } from '@douyinfe/semi-icons';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import './ProductPickGrid.css';

export interface ProductPickItem {
  productId: number;
  productName?: string;
  productPrice?: number | string;
  picUrl?: string;
  picThumb?: string;
}

/** 展示缓存里可能混有旧结构（pic 嵌套 / price / name），读取时统一兼容。 */
type LooseRow = ProductPickItem & {
  pic?: { picUrl?: string; picThumb?: string };
  price?: number | string;
  name?: string;
};

/**
 * 把「选品弹窗带回的行」合并进展示缓存，并严格按 ids 顺序输出。
 * 这样卡片渲染与 ids 永远对齐，拖拽/移除后不会留下孤儿缓存。
 */
export function mergePickedItems(
  ids: number[],
  rows: any[] = [],
  current: any[] = [],
): ProductPickItem[] {
  const cache = new Map<number, ProductPickItem>();
  const normalize = (row: any): ProductPickItem | null => {
    const id = Number(row?.productId);
    if (!Number.isFinite(id) || id <= 0) return null;
    // 展开原对象，保留 marketPrice / productSn 等预览组件可能用到的字段，
    // 只补齐统一的读取字段，避免裁剪掉原有数据结构。
    return {
      ...row,
      productId: id,
      productName: row.productName ?? row.name,
      productPrice: row.productPrice ?? row.price,
      picUrl: row.picUrl ?? row.pic?.picUrl,
      picThumb: row.picThumb ?? row.pic?.picThumb,
    };
  };
  current.forEach((row) => {
    const item = normalize(row);
    if (item) cache.set(item.productId, item);
  });
  rows.forEach((row) => {
    const item = normalize(row);
    if (item) cache.set(item.productId, item);
  });
  return ids
    .map((id) => cache.get(Number(id)))
    .filter((item): item is ProductPickItem => !!item);
}

export interface ProductPickGridProps {
  /** 已选商品 id，顺序即展示顺序（唯一事实来源） */
  ids: number[];
  /** 商品信息展示缓存；缺省时按 ids 自动从接口补全 */
  items?: any[];
  onChange: (ids: number[], items: ProductPickItem[]) => void;
  /** 打开选品弹窗（弹窗及其回填由调用方处理） */
  onRequestPick: () => void;
  /** 分组内等窄容器使用：更小的卡片与间距 */
  compact?: boolean;
  addText?: string;
  className?: string;
}

export default function ProductPickGrid({
  ids,
  items,
  onChange,
  onRequestPick,
  compact = false,
  addText,
  className,
}: ProductPickGridProps) {
  const { t } = useI18n();
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  const list = (Array.isArray(items) ? items : []) as LooseRow[];
  const infoById = new Map<number, LooseRow>();
  list.forEach((row) => {
    const id = Number(row?.productId);
    if (id > 0) infoById.set(id, row);
  });

  // 历史数据只有 ids、没有展示缓存时，按 ids 一次性拉取商品信息补全缩略图/名称/价格。
  // 取数口径与 usePreviewProducts 一致：ids + size + 上架/审核过滤。
  const missingKey = ids.filter((id) => !infoById.has(Number(id))).join(',');
  const fillingRef = useRef(false);
  useEffect(() => {
    if (!missingKey || fillingRef.current) return;
    fillingRef.current = true;
    const missing = missingKey.split(',').map(Number);
    apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>> }>(
      '/product/product/list',
      { ids: missing.join(','), size: missing.length, page: 1, productStatus: 1, checkStatus: 1 },
    )
      .then((res) => {
        const found = res.records || res.list || [];
        if (!found.length) return;
        onChange(ids, mergePickedItems(ids, found, list));
      })
      .catch(() => {
        // 拉取失败保持占位显示，不打断编辑
      })
      .finally(() => {
        fillingRef.current = false;
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [missingKey]);

  const emit = (nextIds: number[], nextList?: LooseRow[]) =>
    onChange(nextIds, mergePickedItems(nextIds, [], nextList ?? list));

  const reorder = (from: number, to: number) => {
    if (from === to) return;
    const next = [...ids];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    emit(next);
  };

  return (
    <div className={`dec-pick${compact ? ' dec-pick--compact' : ''}${className ? ` ${className}` : ''}`}>
      {/* 窄面板里空态不显示工具条：下方虚线卡本身就是引导，省一行空间 */}
      {ids.length > 0 && (
        <div className="dec-pick__bar">
          <span className="dec-pick__count">{t('已选 {count} 个', { count: ids.length })}</span>
          {ids.length > 1 && <span className="dec-pick__tip">{t('拖拽排序')}</span>}
          {confirmClear ? (
            <span className="dec-pick__confirm">
              <button
                type="button"
                className="is-danger"
                onClick={() => { emit([]); setConfirmClear(false); }}
              >
                {t('确定')}
              </button>
              <button type="button" onClick={() => setConfirmClear(false)}>{t('取消')}</button>
            </span>
          ) : (
            <button type="button" className="dec-pick__clear" onClick={() => setConfirmClear(true)}>
              {t('清空')}
            </button>
          )}
        </div>
      )}

      <div className="dec-pick__grid">
        {ids.map((id, index) => {
          const row = infoById.get(Number(id));
          const url = String(row?.picUrl || row?.picThumb || row?.pic?.picUrl || row?.pic?.picThumb || '');
          const name = row?.productName || row?.name || t('商品#{id}', { id });
          const price = row?.productPrice ?? row?.price;
          return (
            <div
              key={String(id)}
              className={`dec-pick__card${dragIndex === index ? ' is-dragging' : ''}${
                overIndex === index && dragIndex !== null && dragIndex !== index ? ' is-drop-target' : ''
              }`}
              draggable
              onDragStart={() => setDragIndex(index)}
              onDragOver={(event) => {
                if (dragIndex === null) return;
                event.preventDefault();
                if (overIndex !== index) setOverIndex(index);
              }}
              onDrop={(event) => {
                event.preventDefault();
                if (dragIndex !== null) reorder(dragIndex, index);
                setDragIndex(null);
                setOverIndex(null);
              }}
              onDragEnd={() => { setDragIndex(null); setOverIndex(null); }}
              title={t('拖拽排序')}
            >
              {url ? (
                <img className="dec-pick__thumb" src={url} alt="" loading="lazy" />
              ) : (
                <span className="dec-pick__thumb dec-pick__thumb--empty">#{id}</span>
              )}
              <span className="dec-pick__badge">{index + 1}</span>
              <button
                type="button"
                className="dec-pick__remove"
                aria-label={t('移除')}
                onClick={(event) => {
                  event.stopPropagation();
                  emit(ids.filter((_, i) => i !== index));
                }}
              >
                ×
              </button>
              <div className="dec-pick__meta">
                <div className="dec-pick__name">{name}</div>
                {price !== undefined && price !== null && (
                  <div className="dec-pick__price">¥{price}</div>
                )}
                <span className="dec-pick__handle" aria-hidden="true">⋮⋮</span>
              </div>
            </div>
          );
        })}

        <button
          type="button"
          className={`dec-pick__add${ids.length === 0 ? ' is-empty' : ''}`}
          onClick={onRequestPick}
        >
          <span className="dec-pick__add-icon"><IconPlus /></span>
          <span className="dec-pick__add-text">
            {ids.length === 0 ? t('点击选择商品') : (addText || t('添加商品'))}
          </span>
        </button>
      </div>
    </div>
  );
}
