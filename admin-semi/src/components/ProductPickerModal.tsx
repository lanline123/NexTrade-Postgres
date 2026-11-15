/**
 * 选择商品弹窗（共享）：卡片网格多选/单选，搜索 + 分页，onOk 按选中顺序回传 ids 与行信息。
 *
 * 2026-11-15 收尾批次从 PcDecorateEditor.tsx 抽出（原为该文件内本地组件，PC 的
 * 商品设置/商品分组/图集等 4 处调用改引本文件，行为逐行等价）；H5 装修编辑器
 * shared/ProductsSourceField 亦改引本弹窗 + ProductPickGrid，弃用旧 ProductPicker
 * （按钮+表格+SideSheet 的另一套选品 UI），两端选品合一。
 * 新增可选 max：PC 调用方不传（不限），H5 来源字段传 max=50 沿旧 ProductPicker 口径。
 * 样式在同目录 ProductPickerModal.css（.dec-prod-picker*，原在 pages/decorate-editor.css，已迁出）。
 */
import { useEffect, useRef, useState } from 'react';
import { Button, Input, Modal, Pagination, Space, Spin, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import './ProductPickerModal.css';

const { Text } = Typography;

export interface ProductRow {
  productId: number;
  productName?: string;
  productPrice?: number | string;
  picUrl?: string;
  picThumb?: string;
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

  useEffect(() => {
    if (visible) {
      setPicked(selectedIds);
      setPage(1);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  const load = (nextPage = page, nextKeyword = keyword) => {
    setLoading(true);
    apiGet<{ records?: ProductRow[]; list?: ProductRow[]; total?: number }>('/product/product/list', {
      keyword: nextKeyword || undefined,
      page: nextPage,
      size: 20,
      productType: 1,
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
    if (visible) load();
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

  return (
    <Modal
      title={t('选择商品')}
      visible={visible}
      width={720}
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
            style={{ width: 260 }}
            showClear
            onClear={() => { setKeyword(''); setPage(1); load(1, ''); }}
          />
          <Button onClick={() => { setPage(1); load(1); }}>{t('搜索')}</Button>
          <span className="dec-prod-picker__count">{t('已选 {count} 个', { count: picked.length })}</span>
        </Space>
        <div className="dec-prod-picker__body">
          {loading && <div className="dec-prod-picker__loading"><Spin /></div>}
          {!loading && rows.length === 0 && <Text type="tertiary">{t('暂无数据')}</Text>}
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
                  {active && <span className="dec-prod-picker__check">✓</span>}
                </div>
                <div className="dec-prod-picker__meta">
                  <div className="dec-prod-picker__name">{row.productName || t('商品#{id}', { id: row.productId })}</div>
                  <div className="dec-prod-picker__price">¥{row.productPrice ?? '-'}</div>
                </div>
              </div>
            );
          })}
        </div>
        {total > 20 && (
          <div className="dec-prod-picker__pager">
            <Pagination
              currentPage={page}
              pageSize={20}
              total={total}
              onPageChange={(p) => { setPage(p); load(p); }}
              size="small"
            />
          </div>
        )}
      </div>
    </Modal>
  );
}
