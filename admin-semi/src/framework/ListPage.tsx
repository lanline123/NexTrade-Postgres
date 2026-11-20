import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import {
  Button,
  Card,
  DatePicker,
  Input,
  Popconfirm,
  Popover,
  Select,
  Space,
  Table,
  Toast,
} from '@douyinfe/semi-ui';
import { IconClose, IconPlus, IconRefresh, IconSearch } from '@douyinfe/semi-icons';
import { apiGet, apiPost, isDemo } from '../api';
import { useI18n } from '../i18n';
import type { FilterConfig, ListPageProps } from './types';
import FormDrawer from './FormDrawer';
import TagFilter from './TagFilter';

function renderFilter(
  filter: FilterConfig,
  value: any,
  onChange: (value: any) => void,
  t: (key: string, params?: Record<string, string | number>) => string,
) {
  const style = { width: filter.width || 180 };
  const rawPlaceholder = filter.placeholder || filter.label;
  const translatePlaceholder = (): string => {
    const direct = t(rawPlaceholder);
    if (direct !== rawPlaceholder) return direct;
    const patterns: Array<[string, string]> = [
      ['请输入', '请输入{label}'],
      ['输入', '输入{label}'],
      ['请选择', '请选择{label}'],
      ['搜索', '搜索{label}'],
    ];
    for (const [prefix, key] of patterns) {
      if (rawPlaceholder.startsWith(prefix)) {
        return t(key, { label: t(rawPlaceholder.slice(prefix.length)) });
      }
    }
    return rawPlaceholder;
  };
  const placeholder = translatePlaceholder();
  if (filter.type === 'tags') {
    return <TagFilter value={value} onChange={onChange} width={Number(filter.width) || 220} />;
  }
  if (filter.type === 'select') {
    return (
      <Select
        placeholder={placeholder}
        value={value}
        optionList={(filter.options || []).map((option) => ({ ...option, label: t(String(option.label)) }))}
        onChange={onChange}
        style={style}
      />
    );
  }
  if (filter.type === 'date') {
    return <DatePicker placeholder={placeholder} value={value} onChange={onChange} style={style} />;
  }
  if (filter.type === 'number') {
    return (
      <Input
        placeholder={placeholder}
        value={value ?? ''}
        onChange={(next) => onChange(next)}
        style={style}
      />
    );
  }
  return (
    <Input
      prefix={<IconSearch />}
      placeholder={placeholder}
      value={value ?? ''}
      onChange={(next) => onChange(next)}
      onEnterPress={() => onChange(value)}
      style={style}
    />
  );
}

export default function ListPage<T extends Record<string, any>>({
  api,
  listMethod = 'get',
  rowKey,
  columns,
  filters = [],
  query,
  pageSize = 10,
  drawer,
  showEdit = true,
  actionWidth = 160,
  selectable = false,
  rowSelectable,
  batchActions,
  filterLayout = 'inline',
  onDataLoaded,
  onReset,
  editRequest,
  onEditRequestHandled,
  toolbarExtra,
  cardTitle,
  onRowClick,
  rowActions,
}: ListPageProps<T>) {
  const { t } = useI18n();
  const [rows, setRows] = useState<T[]>([]);
  const resolvedRowKey =
    typeof rowKey === 'function' ? '__frameworkKey' : (rowKey as string);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(!isDemo());
  const [reloadKey, setReloadKey] = useState(0);
  const [filterValues, setFilterValues] = useState<Record<string, any>>({});
  const [selectedKeys, setSelectedKeys] = useState<Array<string | number>>([]);
  const cardRef = useRef<HTMLDivElement | null>(null);
  const [containerWidth, setContainerWidth] = useState(0);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [drawerMode, setDrawerMode] = useState<'create' | 'edit'>('create');
  const [drawerInitial, setDrawerInitial] = useState<Record<string, any>>({});
  const [editingRecord, setEditingRecord] = useState<T | null>(null);
  const [saving, setSaving] = useState(false);

  const filterMap = useMemo(
    () => Object.fromEntries(filters.map((item) => [item.field, item])),
    [filters],
  );

  const tableRows = useMemo(
    () =>
      rows.map((row, index) => ({
        ...(row as Record<string, any>),
        __frameworkKey:
          typeof rowKey === 'function'
            ? (rowKey as (row: T) => string)(row)
            : String((row as Record<string, any>)[rowKey as string] ?? index),
      })),
    [rows, rowKey],
  );

  // ===== 每页条数按浏览器高度自适应（我方新增，原版为固定条数）=====
  // 口径：pageSize prop 只作首帧量测前与量测失败时的兜底；量测完成后一律按
  // 「表格可用高度 ÷ 真实行高」取条数，下限 5（用户口径）、上限 50（防超大页拖垮接口与渲染）。
  // 可用高度 = 滚动容器（.page-content）可视底边 − 表格顶 − 表头 − 分页器 − 卡片底部留白。
  const ADAPTIVE_MIN_SIZE = 5;
  const ADAPTIVE_MAX_SIZE = 50;
  const FALLBACK_ROW_HEIGHT = 48; // 单元格上下 padding 12+12 + 内容行高约 21 + 边框 1 ≈ 46~48
  const FALLBACK_HEAD_HEIGHT = 41;
  const FALLBACK_PAGINATION_HEIGHT = 56;
  const [adaptiveSize, setAdaptiveSize] = useState<number | null>(null);
  const [sizeReady, setSizeReady] = useState(false);
  const adaptiveSizeRef = useRef<number | null>(null);
  const rowsCountRef = useRef(0);
  rowsCountRef.current = tableRows.length; // 量测函数只读 ref，保证首帧注册的监听回调永远量到最新行数/行高
  const rowHeightRef = useRef(0);
  const tableWrapRef = useRef<HTMLDivElement | null>(null);
  const measureTimerRef = useRef<number | null>(null);
  const effectivePageSize = adaptiveSize ?? pageSize;

  const computeAdaptiveSize = (): number | null => {
    const wrap = tableWrapRef.current;
    if (!wrap || typeof window === 'undefined') return null;
    const wrapRect = wrap.getBoundingClientRect();
    if (wrapRect.width === 0) return null; // 隐藏态（未激活页签等）不量测，可见时由 ResizeObserver 补量
    const container = wrap.closest('.page-content') as HTMLElement | null;
    let visibleBottom: number;
    let wrapTop: number;
    if (container) {
      const containerRect = container.getBoundingClientRect();
      wrapTop = wrapRect.top - containerRect.top + container.scrollTop;
      visibleBottom = container.scrollTop + container.clientHeight - 20; // 内容区底部 padding 20
    } else {
      wrapTop = wrapRect.top + window.scrollY;
      visibleBottom = window.scrollY + window.innerHeight;
    }
    const headEl = wrap.querySelector('.semi-table-thead');
    const headHeight = headEl?.getBoundingClientRect().height || FALLBACK_HEAD_HEIGHT;
    const paginationEl = wrap.querySelector('.semi-table-pagination-outer');
    const paginationHeight = paginationEl
      ? paginationEl.getBoundingClientRect().height + 16
      : FALLBACK_PAGINATION_HEIGHT;
    // 行高以真实首行为准（带图行更高，按本页实际行高算）；量不到沿用上次量测/兜底常量
    if (rowsCountRef.current > 0) {
      const firstRow = wrap.querySelector('.semi-table-tbody .semi-table-row');
      const measured = firstRow?.getBoundingClientRect().height || 0;
      if (measured >= 24 && measured <= 240) rowHeightRef.current = measured;
    }
    const rowHeight = rowHeightRef.current || FALLBACK_ROW_HEIGHT;
    const bodyHeight = visibleBottom - wrapTop - headHeight - paginationHeight - 26; // 26 = 卡片底部 padding 18 + 安全余量 8
    if (bodyHeight <= 0) return null;
    const size = Math.floor(bodyHeight / rowHeight);
    return Math.min(ADAPTIVE_MAX_SIZE, Math.max(ADAPTIVE_MIN_SIZE, size));
  };

  const applyMeasure = () => {
    const next = computeAdaptiveSize();
    setSizeReady(true);
    if (next != null && next !== adaptiveSizeRef.current) {
      adaptiveSizeRef.current = next;
      setAdaptiveSize(next);
      setPage(1); // 条数变化回到第 1 页重拉
    }
  };

  const scheduleMeasure = () => {
    if (typeof window === 'undefined') return;
    if (measureTimerRef.current != null) window.clearTimeout(measureTimerRef.current);
    measureTimerRef.current = window.setTimeout(applyMeasure, 120);
  };

  // 首帧量测赶在绘制前（layout effect 先于 fetch 的 passive effect），
  // fetch 以 sizeReady 为门：首拉即带自适应条数，不先按兜底值白拉一遍。
  useLayoutEffect(() => {
    applyMeasure();
  }, []);

  useEffect(() => {
    const onResize = () => scheduleMeasure();
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // 数据渲染后按真实行高复量（带图行与普通行行高不同，首帧只能用兜底行高）
  useEffect(() => {
    scheduleMeasure();
  }, [rows]);

  useEffect(() => {
    if (isDemo()) return;
    if (!sizeReady) return;
    let alive = true;
    setLoading(true);
    const params = {
      page,
      size: effectivePageSize,
      ...query,
      ...filterValues,
    };
    const request =
      listMethod === 'post'
        ? apiPost<{ records: T[]; total: number }>(api, params)
        : apiGet<{ records: T[]; total: number }>(api, params);
    request
      .then((result) => {
        if (!alive) return;
        setRows(result.records || []);
        setTotal(result.total || 0);
        onDataLoaded?.({ total: result.total || 0, rows: result.records || [] });
      })
      .catch((error) => alive && Toast.error({ content: error.message }))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [api, listMethod, page, effectivePageSize, sizeReady, query, filterValues, reloadKey]);

  const search = () => {
    setPage(1);
    setReloadKey((key) => key + 1);
  };

  const reset = () => {
    setFilterValues({});
    setPage(1);
    setReloadKey((key) => key + 1);
    // 外部持有的筛选（如店铺）同步回全部，保证「重置」语义一致
    onReset?.();
  };

  const openCreate = () => {
    if (!drawer) return;
    setDrawerMode('create');
    setEditingRecord(null);
    setDrawerInitial(drawer.sections.reduce<Record<string, any>>((acc, section) => {
      section.fields.forEach((field) => {
        if (field.initValue !== undefined) {
          acc[field.field] = field.initValue;
        } else if (field.type === 'switch') {
          acc[field.field] = false;
        }
      });
      return acc;
    }, {}));
    setDrawerVisible(true);
  };

  const openEdit = async (record: T) => {
    if (!drawer) return;
    setDrawerMode('edit');
    setEditingRecord(record);
    if (drawer.detailApi && drawer.idField) {
      try {
        const detail = await apiGet<Record<string, any>>(drawer.detailApi, {
          id: (record as Record<string, any>)[resolvedRowKey],
        });
        setDrawerInitial(detail || record);
      } catch {
        setDrawerInitial(record);
      }
    } else {
      setDrawerInitial(record);
    }
    setDrawerVisible(true);
  };

  useEffect(() => {
    if (!editRequest) return;
    void openEdit(editRequest);
    onEditRequestHandled?.();
  }, [editRequest]);

  const save = async (payload: Record<string, any>) => {
    if (!drawer) return;
    setSaving(true);
    try {
      const targetApi =
        drawerMode === 'create' ? drawer.createApi : drawer.updateApi || drawer.createApi;
      const body =
        drawerMode === 'edit' && drawer.idField
          ? { ...payload, [drawer.idField]: (editingRecord as Record<string, any>)?.[resolvedRowKey] }
          : payload;
      await apiPost(targetApi, body);
      Toast.success({ content: drawerMode === 'create' ? t('创建成功') : t('更新成功') });
      setDrawerVisible(false);
      setPage(1);
      setReloadKey((key) => key + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const remove = async (record: T) => {
    if (!drawer?.deleteApi) return;
    try {
      await apiPost(drawer.deleteApi, { id: (record as Record<string, any>)[resolvedRowKey] });
      Toast.success({ content: t('删除成功') });
      setReloadKey((key) => key + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  useEffect(() => {
    const element = cardRef.current;
    if (!(element instanceof Element)) return;
    const observer = new ResizeObserver((entries) => {
      setContainerWidth(entries[0]?.contentRect.width || 0);
      // 卡片尺寸变化（窗口缩放、筛选 chip 行增减、页签由隐藏转可见）都可能改变可用高度，防抖复量条数
      scheduleMeasure();
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const hasActions = Boolean(drawer || rowActions);
  const selectedRows = rows.filter((row) =>
    selectedKeys.includes((row as Record<string, any>)[resolvedRowKey]),
  );
  const activeEntries = Object.entries(filterValues).filter(
    ([, value]) => value !== '' && value !== undefined && value !== null,
  );
  const displayFilterValue = (field: string, value: any) => {
    const config = filterMap[field];
    const option = config?.options?.find((item) => item.value === value);
    return option ? `：${t(String(option.label))}` : `：${value}`;
  };
  // 勾选列不在 columns 内（Semi rowSelection 另加一列，默认宽 60），必须计入
  // 总宽：漏算时临界宽度下 scroll 判断偏小，表格溢出会漏到页面层冒出横向滚动条
  const scrollX =
    columns.reduce((sum, column) => sum + (column.width || 150), 0) +
    (hasActions ? actionWidth : 0) +
    (selectable ? 60 : 0);
  const localizedColumns = columns.map((column) => ({
    ...column,
    title: typeof column.title === 'string' ? t(column.title) : column.title,
  }));
  const tableColumns = [
    ...localizedColumns,
    ...(hasActions
      ? [
          {
            title: t('操作'),
            width: actionWidth,
            fixed: 'right',
            render: (_: unknown, record: T) => (
              <Space spacing={2}>
                {drawer && showEdit && (
                  <Button theme="borderless" size="small" onClick={() => openEdit(record)}>
                    {t('编辑')}
                  </Button>
                )}
                {drawer?.deleteApi && (
                  <Popconfirm title={t('确定删除？')} onConfirm={() => remove(record)}>
                    <Button theme="borderless" type="danger" size="small">
                      {t('删除')}
                    </Button>
                  </Popconfirm>
                )}
                {rowActions?.(record as T)}
              </Space>
            ),
          },
        ]
      : []),
  ];

  return (
    <>
      <div ref={cardRef}>
        <Card className="table-card" shadows="hover">
        <div className={`table-toolbar ${filterLayout === 'popover' ? 'is-popover' : ''}`}>
          <Space>
            {filters[0] && (
              <span className="framework-filter">
                {renderFilter(filterMap[filters[0].field], filterValues[filters[0].field], (value) =>
                  setFilterValues((current) => ({ ...current, [filters[0].field]: value })),
                t)}
              </span>
            )}
            {filters[0] && (
              <Button theme="solid" type="primary" icon={<IconSearch />} onClick={search}>
                {t('搜索')}
              </Button>
            )}
            {filterLayout === 'popover' && filters.length > 1 ? (
              <Popover
                trigger="click"
                position="bottomLeft"
                content={
                  <div className="framework-filter-popover">
                    {filters.slice(1).map((filter) => (
                      <div className="framework-filter-row" key={filter.field}>
                        <label>{t(filter.label)}</label>
                        {renderFilter(filter, filterValues[filter.field], (value) =>
                          setFilterValues((current) => ({ ...current, [filter.field]: value })),
                        t)}
                      </div>
                    ))}
                    <div className="framework-filter-actions">
                      <Button onClick={reset}>{t('重置')}</Button>
                      <Button theme="solid" type="primary" onClick={search}>
                        {t('应用筛选')}
                      </Button>
                    </div>
                  </div>
                }
              >
                <Button>
                  {t('筛选')}
                  {activeEntries.length > 0 ? ` (${activeEntries.length})` : ''}
                </Button>
              </Popover>
            ) : (
              <>
                {filters.slice(1).map((filter) => (
                  <span key={filter.field} className="framework-filter">
                    {renderFilter(filter, filterValues[filter.field], (value) =>
                      setFilterValues((current) => ({ ...current, [filter.field]: value })),
                    t)}
                  </span>
                ))}
                {filters.length > 0 && (
                  <Button onClick={reset}>{t('重置')}</Button>
                )}
              </>
            )}
          </Space>
          <Space>
            {toolbarExtra}
            <Button icon={<IconRefresh />} onClick={() => setReloadKey((key) => key + 1)}>
              {t('刷新')}
            </Button>
            {drawer && (
              <Button theme="solid" type="primary" icon={<IconPlus />} onClick={openCreate}>
                {t(drawer.addText || '新增')}
              </Button>
            )}
          </Space>
        </div>

        {filterLayout === 'popover' && activeEntries.length > 0 && (
          <div className="framework-active-filters">
            <span className="framework-active-label">{t('已选条件')}</span>
            {activeEntries.map(([field, value]) => (
              <span className="framework-filter-chip" key={field}>
                {t(filterMap[field]?.label || field)}
                {displayFilterValue(field, value)}
                <IconClose
                  size="small"
                  onClick={() =>
                    setFilterValues((current) => {
                      const next = { ...current };
                      delete next[field];
                      return next;
                    })
                  }
                />
              </span>
            ))}
            <button type="button" onClick={reset}>
              {t('清空')}
            </button>
          </div>
        )}

        {batchActions && selectedRows.length > 0 && (
          <div className="batch-floating-bar">
            <span>{t('已选择 {count} 项', { count: selectedRows.length })}</span>
            {batchActions(selectedRows, () => setSelectedKeys([]))}
            <Button theme="borderless" size="small" onClick={() => setSelectedKeys([])}>
              {t('取消选择')}
            </Button>
          </div>
        )}

        <div ref={tableWrapRef}>
        <Table
          loading={loading}
          dataSource={tableRows}
          rowKey={resolvedRowKey}
          rowSelection={
            selectable
              ? {
                  selectedRowKeys: selectedKeys,
                  onChange: (keys) => setSelectedKeys((keys || []) as Array<string | number>),
                  // 逐行禁用勾选（如默认页不可批删）：rowSelectable 返回 false 的行
                  // 勾选框禁用、全选亦不纳入（Semi getCheckboxProps 口径）
                  ...(rowSelectable
                    ? {
                        getCheckboxProps: (record: Record<string, any>) => ({
                          disabled: !rowSelectable(record as T),
                        }),
                      }
                    : {}),
                }
              : undefined
          }
          onRow={onRowClick ? (record) => ({ onClick: () => onRowClick(record as unknown as T) }) : undefined}
          scroll={containerWidth > 0 && scrollX > containerWidth - 48 ? { x: scrollX } : undefined}
          pagination={{
            currentPage: page,
            pageSize: effectivePageSize,
            total,
            onChange: (nextPage: number) => setPage(nextPage),
          }}
          columns={tableColumns as any}
        />
        </div>
      </Card>
      </div>

      {drawer && (
        <FormDrawer
          visible={drawerVisible}
          mode={drawerMode}
          initialValues={drawerInitial}
          config={drawer}
          saving={saving}
          onCancel={() => setDrawerVisible(false)}
          onSubmit={save}
        />
      )}
    </>
  );
}
