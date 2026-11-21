/**
 * 商品选择器（共享组件）：左侧商品类目栏 + 右侧商品卡片网格，多选/单选，
 * 类目筛选 + 搜索 + 分页，onOk 按选中顺序回传 ids 与行信息。
 *
 * 2026-11-15 收尾批次从 PcDecorateEditor.tsx 抽出（原为该文件内本地组件，PC 的
 * 商品设置/商品分组/图集等 4 处调用改引本文件，行为逐行等价）；H5 装修编辑器
 * shared/ProductsSourceField 亦改引本弹窗 + ProductPickGrid，弃用旧 ProductPicker
 * （按钮+表格+SideSheet 的另一套选品 UI），两端选品合一。
 * 新增可选 max：PC 调用方不传（不限），H5 来源字段传 max=50 沿旧 ProductPicker 口径。
 * 样式在同目录 ProductPickerModal.css（.dec-prod-picker*，原在 pages/decorate-editor.css，已迁出）。
 *
 * 2026-11-21 现代化改版（用户在 10023 页提出）：弹窗加宽至 1120、新增左侧类目栏、
 * 每页 20 → 40、卡片改扁平直角 + 主题色选中态。对外 API（props / ProductRow /
 * onOk 回传顺序语义）逐字不变，全部调用点零改动。
 * 类目取数：GET /product/category/getAllCategory（现行类目树接口，节点
 * categoryId / categoryName / children）；商品按类目筛选走后端真参数 categoryId
 * （AdminProductListQuery.category_id，服务层展开全部子孙类目、仓储层 IN 过滤，
 * 非前端假筛选）。搜索口径：关键词与当前类目**叠加**（在当前类目内搜索），
 * 切到「全部商品」即全局搜索；切换类目时页码归 1。
 */
import { useEffect, useRef, useState } from 'react';
import { Button, Input, Modal, Pagination, Space, Spin, Toast, Typography } from '@douyinfe/semi-ui';
import { IconChevronDown, IconChevronRight, IconTick } from '@douyinfe/semi-icons';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import './ProductPickerModal.css';

const { Text } = Typography;

/** 每页商品数（2026-11-21 改版由 20 提升至 40，卡片更紧凑多列展示） */
const PAGE_SIZE = 40;

export interface ProductRow {
  productId: number;
  productName?: string;
  productPrice?: number | string;
  picUrl?: string;
  picThumb?: string;
}

interface CategoryNode {
  categoryId: number;
  categoryName: string;
  children: CategoryNode[];
}

function normalizeCategoryTree(nodes: any): CategoryNode[] {
  if (!Array.isArray(nodes)) return [];
  return nodes
    .map((node) => ({
      categoryId: Number(node?.categoryId),
      categoryName: String(node?.categoryName || ''),
      children: normalizeCategoryTree(node?.children),
    }))
    .filter((node) => Number.isFinite(node.categoryId) && node.categoryId > 0);
}

export default function ProductPickerModal({
  visible,
  selectedIds,
  multiple = true,
  max,
  onOk,
  onCancel,
}: {
  visible: boolean;
  selectedIds: number[];
  multiple?: boolean;
  /** 最多可选数量（不传 = 不限）；H5 商品来源字段沿旧口径传 50 */
  max?: number;
  onOk: (ids: number[], rows: ProductRow[]) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<ProductRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [picked, setPicked] = useState<number[]>(selectedIds);
  /** 已选行的信息缓存：onOk 时按 picked 顺序回传，调用方（商品/图集/主图卡）才能拿到缩略图与价格。 */
  const rowsByIdRef = useRef<Map<number, ProductRow>>(new Map());
  /** 左侧类目栏：0 = 全部商品；类目树每次打开时拉取，失败只保留「全部商品」不阻断选品。 */
  const [categories, setCategories] = useState<CategoryNode[]>([]);
  const [categoryId, setCategoryId] = useState(0);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  const load = (nextPage = page, nextKeyword = keyword, nextCategoryId = categoryId) => {
    setLoading(true);
    apiGet<{ records?: ProductRow[]; list?: ProductRow[]; total?: number }>('/product/product/list', {
      keyword: nextKeyword || undefined,
      page: nextPage,
      size: PAGE_SIZE,
      productType: 1,
      categoryId: nextCategoryId > 0 ? nextCategoryId : undefined,
    })
      .then((res) => {
        const list = res.records || res.list || [];
        list.forEach((row) => rowsByIdRef.current.set(row.productId, row));
        setRows(list);
        setTotal(Number(res.total ?? list.length));
      })
      .catch(() => setRows([]))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (visible) {
      setPicked(selectedIds);
      setPage(1);
      setCategoryId(0);
      load(1, keyword, 0);
      apiGet<any>('/product/category/getAllCategory')
        .then((tree) => {
          const normalized = normalizeCategoryTree(tree);
          setCategories(normalized);
          // 默认展开第一层类目，子类目按需点开
          setExpandedIds(new Set(normalized.map((node) => node.categoryId)));
        })
        .catch(() => setCategories([]));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  const toggle = (row: ProductRow) => {
    rowsByIdRef.current.set(row.productId, row);
    setPicked((prev) => {
      if (prev.includes(row.productId)) return prev.filter((id) => id !== row.productId);
      if (!multiple) return [row.productId];
      if (max && prev.length >= max) {
        Toast.warning({ content: t('最多选择 {count} 个商品', { count: max }) });
        return prev;
      }
      return [...prev, row.productId];
    });
  };

  const selectCategory = (id: number) => {
    if (id === categoryId) return;
    setCategoryId(id);
    setPage(1);
    load(1, keyword, id);
  };

  const toggleExpand = (id: number) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  /** 把类目树按展开状态压平为可见行（带层级深度），供左侧栏渲染。 */
  const visibleCategories: Array<{ node: CategoryNode; depth: number }> = [];
  const walkCategories = (nodes: CategoryNode[], depth: number) => {
    nodes.forEach((node) => {
      visibleCategories.push({ node, depth });
      if (node.children.length && expandedIds.has(node.categoryId)) {
        walkCategories(node.children, depth + 1);
      }
    });
  };
  walkCategories(categories, 0);

  return (
    <Modal
      title={t('选择商品')}
      visible={visible}
      width={1120}
      onOk={() =>
        onOk(
          picked,
          picked
            .map((id) => rowsByIdRef.current.get(id))
            .filter((row): row is ProductRow => !!row),
        )
      }
      onCancel={onCancel}
      okText={t('确定')}
      cancelText={t('取消')}
    >
      <div className="dec-prod-picker">
        <Space style={{ marginBottom: 12, width: '100%' }} align="center">
          <Input
            value={keyword}
            onChange={setKeyword}
            onEnterPress={() => { setPage(1); load(1); }}
            placeholder={t('输入商品名称搜索')}
            style={{ width: 300 }}
            showClear
            onClear={() => { setKeyword(''); setPage(1); load(1, ''); }}
          />
          <Button onClick={() => { setPage(1); load(1); }}>{t('搜索')}</Button>
          <span className="dec-prod-picker__count">{t('已选 {count} 个', { count: picked.length })}</span>
        </Space>
        <div className="dec-prod-picker__layout">
          <div className="dec-prod-picker__cats">
            <div
              className={`dec-prod-picker__cat${categoryId === 0 ? ' is-active' : ''}`}
              onClick={() => selectCategory(0)}
            >
              <span className="dec-prod-picker__cat-name">{t('全部商品')}</span>
            </div>
            {visibleCategories.map(({ node, depth }) => {
              const hasChildren = node.children.length > 0;
              const expanded = expandedIds.has(node.categoryId);
              return (
                <div
                  key={node.categoryId}
                  className={`dec-prod-picker__cat${categoryId === node.categoryId ? ' is-active' : ''}`}
                  style={{ paddingLeft: 12 + depth * 16 }}
                  onClick={() => selectCategory(node.categoryId)}
                  title={node.categoryName}
                >
                  {hasChildren ? (
                    <span
                      className="dec-prod-picker__cat-arrow"
                      onClick={(event) => { event.stopPropagation(); toggleExpand(node.categoryId); }}
                    >
                      {expanded ? <IconChevronDown size="small" /> : <IconChevronRight size="small" />}
                    </span>
                  ) : (
                    <span className="dec-prod-picker__cat-arrow is-empty" />
                  )}
                  <span className="dec-prod-picker__cat-name">{node.categoryName}</span>
                </div>
              );
            })}
          </div>
          <div className="dec-prod-picker__main">
            <div className="dec-prod-picker__body">
              {loading && <div className="dec-prod-picker__loading"><Spin /></div>}
              {!loading && rows.length === 0 && (
                <div className="dec-prod-picker__empty"><Text type="tertiary">{t('暂无数据')}</Text></div>
              )}
              {!loading && rows.map((row) => {
                const active = picked.includes(row.productId);
                const url = String(row.picThumb || row.picUrl || '');
                return (
                  <div
                    key={row.productId}
                    className={`dec-prod-picker__card${active ? ' is-active' : ''}`}
                    onClick={() => toggle(row)}
                    title={row.productName || t('商品#{id}', { id: row.productId })}
                  >
                    <div className="dec-prod-picker__pic">
                      {url ? <img src={url} alt="" loading="lazy" /> : <span className="dec-prod-picker__pic-empty">#{row.productId}</span>}
                      {active && <span className="dec-prod-picker__check"><IconTick size="small" /></span>}
                    </div>
                    <div className="dec-prod-picker__meta">
                      <div className="dec-prod-picker__name">{row.productName || t('商品#{id}', { id: row.productId })}</div>
                      <div className="dec-prod-picker__price">¥{row.productPrice ?? '-'}</div>
                    </div>
                  </div>
                );
              })}
            </div>
            <div className="dec-prod-picker__pager">
              <span className="dec-prod-picker__total">{t('共 {count} 个商品', { count: total })}</span>
              {total > PAGE_SIZE && (
                <Pagination
                  currentPage={page}
                  pageSize={PAGE_SIZE}
                  total={total}
                  onPageChange={(p) => { setPage(p); load(p); }}
                  size="small"
                />
              )}
            </div>
          </div>
        </div>
      </div>
    </Modal>
  );
}
