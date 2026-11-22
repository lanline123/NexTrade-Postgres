import { useEffect, useState } from 'react';
import {
  Button,
  Card,
  DatePicker,
  Empty,
  Input,
  Modal,
  Radio,
  RadioGroup,
  Select,
  Space,
  Spin,
  Table,
  Tabs,
  Tag,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconChevronDown, IconImage, IconRefresh, IconSearch } from '@douyinfe/semi-icons';
import { apiGet, apiPost, isDemo } from '../api';
import { useI18n } from '../i18n';
import { mockOrderPage } from '../mock';
import type { OrderRow, PageResult } from '../types';
import OrderDetailModal from '../components/OrderDetailModal';
import ProactiveRefundModal from '../components/ProactiveRefundModal';
import { WriteOffModal } from '../components/OrderExtraDialogs';
import {
  BatchDeliverImportModal,
  BatchShipModal,
  BatchSignModal,
  ChangeStatusModal,
} from '../components/OrderBatchDialogs';
import './orders.css';

const { Text } = Typography;

const statusTabs = [
  { tab: '全部', key: '' },
  { tab: '待付款', key: '0' },
  { tab: '待发货', key: '1' },
  { tab: '待收货', key: '2' },
  { tab: '已完成', key: '5' },
  { tab: '已取消', key: '3' },
];

/** 订单状态配色按码值（后端 OrderStatus：0 待支付/1 已确认待发货/2 处理中已发货/
 * 3 已取消/4 无效/5 已完成）。原实现按文案 key 匹配，但后端下发文案是
 * 「待支付/已确认，待发货/处理中，已发货」，与 key 全对不上，配色恒落默认蓝。 */
const statusColorByCode: Record<number, string> = {
  0: 'orange',
  1: 'blue',
  2: 'cyan',
  3: 'grey',
  4: 'grey',
  5: 'green',
};

type OrderFilters = {
  keyword: string;
  outTradeNo: string;
  address: string;
  email: string;
  mobile: string;
  payStatus: number | undefined;
  shippingStatus: number | undefined;
  addTime: string[];
  mark: number; // -1 不限（旧版「订单标记」筛选）
  isGift: number | undefined; // 送礼物筛选（我方新增）：1 只看礼物订单
  deliveryDate: string; // 送达日期筛选（我方新增，论坛帖 467744）：yyyy-MM-dd，空 = 不筛
};

const emptyFilters: OrderFilters = {
  keyword: '',
  outTradeNo: '',
  address: '',
  email: '',
  mobile: '',
  payStatus: undefined,
  shippingStatus: undefined,
  addTime: [],
  mark: -1,
  isGift: undefined,
  deliveryDate: '',
};

/** 会员标签筛选状态（对齐旧版「按会员标签筛选订单」弹窗）。 */
type MemberTagFilter = {
  memberTagIds: number[];
  memberTagMatchMode: number; // 0 任一 / 1 全部
};

const emptyMemberTagFilter: MemberTagFilter = { memberTagIds: [], memberTagMatchMode: 0 };

/** 组合搜索字段定义（搜索栏改版）：label/placeholder 为 i18n key，渲染时经 t() 翻译。 */
type ComboField = 'keyword' | 'outTradeNo' | 'mobile' | 'email' | 'address';
const comboFieldDefs: Array<{ field: ComboField; label: string; placeholder: string }> = [
  { field: 'keyword', label: '综合', placeholder: '搜索订单号 / 收货人 / 商品名称' },
  { field: 'outTradeNo', label: '交易单号', placeholder: '请输入交易单号' },
  { field: 'mobile', label: '手机号', placeholder: '手机号' },
  { field: 'email', label: '邮箱', placeholder: 'Email' },
  { field: 'address', label: '地址', placeholder: '详细地址' },
];

/** 商品缩略图：无图 / 加载失败时渲染规范占位，不裂图。 */
function ProductThumb({ src, size }: { src?: string; size: number }) {
  const [failed, setFailed] = useState(false);
  if (!src || failed) {
    return (
      <div className="order-thumb order-thumb-empty" style={{ width: size, height: size }}>
        <IconImage size={size <= 24 ? 'small' : undefined} />
      </div>
    );
  }
  return (
    <img
      className="order-thumb"
      src={src}
      alt=""
      width={size}
      height={size}
      loading="lazy"
      onError={() => setFailed(true)}
    />
  );
}

/** 订单商品主体区（改版新增）：首件图 + 标题 + 规格，其余件小图条与共 N 件摘要；点击打开详情。 */
function OrderProductCell({ record, onOpen }: { record: OrderRow; onOpen: () => void }) {
  const { t } = useI18n();
  const items = record.items || [];
  const first = items[0];
  if (!first) return <Text type="tertiary">-</Text>;
  const totalQty = items.reduce((sum, item) => sum + (item.quantity || 0), 0);
  return (
    <div className="order-product-cell" onClick={onOpen} title={t('查看详情')}>
      <ProductThumb src={first.picThumb} size={48} />
      <div className="order-product-main">
        <div className="order-product-name">{first.productName || '-'}</div>
        <div className="order-product-meta">
          {first.skuValue ? <span className="order-product-sku">{first.skuValue}</span> : null}
          <span>×{first.quantity ?? 1}</span>
        </div>
        {items.length > 1 ? (
          <div className="order-product-more">
            <span className="order-mini-thumbs">
              {items.slice(1, 5).map((item, idx) => (
                <ProductThumb key={item.itemId ?? idx} src={item.picThumb} size={20} />
              ))}
            </span>
            <Text type="tertiary" size="small">
              {t('共')} {totalQty} {t('件')} · {items.length} {t('种商品')}
            </Text>
          </div>
        ) : null}
      </div>
    </div>
  );
}

export default function Orders({ onNavigate }: { onNavigate?: (page: string) => void } = {}) {
  const { t } = useI18n();
  const [rows, setRows] = useState<OrderRow[]>(isDemo() ? mockOrderPage.records : []);
  const [total, setTotal] = useState(isDemo() ? mockOrderPage.total : 0);
  const [page, setPage] = useState(1);
  const [status, setStatus] = useState('');
  const [filters, setFilters] = useState<OrderFilters>(emptyFilters);
  const [applied, setApplied] = useState<OrderFilters>(emptyFilters);
  const [loading, setLoading] = useState(!isDemo());
  const [reloadKey, setReloadKey] = useState(0);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailId, setDetailId] = useState<number | undefined>(undefined);
  // 主动退款【我方新增】：已支付且待发货/已发货/已完成的订单可发起
  const [proactiveOrderId, setProactiveOrderId] = useState<number | null>(null);
  const [writeOffOpen, setWriteOffOpen] = useState(false);
  // 批量操作（对齐旧版：添加标记 / 订单批量发货 / Excel批量发货）
  const [selectedIds, setSelectedIds] = useState<Array<string | number>>([]);
  const [batchSignOpen, setBatchSignOpen] = useState(false);
  const [batchShipOpen, setBatchShipOpen] = useState(false);
  const [batchImportOpen, setBatchImportOpen] = useState(false);
  const [logistics, setLogistics] = useState<Array<{ id: number; name: string }>>([]);
  // 行级取消订单 / 取消发货（旧版 ChangeStatus，preOrderStatus 3/1）
  const [changeStatus, setChangeStatus] = useState<{ orderId: number; preOrderStatus: number; title: string } | null>(null);
  // 按会员标签筛选
  const [memberTagFilter, setMemberTagFilter] = useState<MemberTagFilter>(emptyMemberTagFilter);
  const [appliedMemberTagFilter, setAppliedMemberTagFilter] = useState<MemberTagFilter>(emptyMemberTagFilter);
  const [memberTagDialogOpen, setMemberTagDialogOpen] = useState(false);
  const [tagGroups, setTagGroups] = useState<Array<{ groupId: number; groupName: string; tags: Array<{ id: number; tagName: string }> }>>([]);
  const [tagLoading, setTagLoading] = useState(false);
  // 搜索栏改版：组合搜索当前字段 / 高级筛选展开 / 批量订单号弹窗
  const [comboField, setComboField] = useState<ComboField>('keyword');
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [batchSnOpen, setBatchSnOpen] = useState(false);
  const [batchSnText, setBatchSnText] = useState('');

  useEffect(() => {
    if (isDemo()) return;
    // 物流公司（批量发货弹窗用）
    apiGet<{ records: Array<{ logisticsId: number; logisticsName: string }> }>('/setting/logisticsCompany/list', { page: 1, size: 100 })
      .then((result) => setLogistics((result.records || []).map((item) => ({ id: item.logisticsId, name: item.logisticsName }))))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    if (isDemo()) return;
    let alive = true;
    setLoading(true);
    apiGet<PageResult<OrderRow>>('/order/order/list', {
      page,
      size: 10,
      orderStatus: status || undefined,
      keyword: applied.keyword || undefined,
      outTradeNo: applied.outTradeNo || undefined,
      address: applied.address || undefined,
      email: applied.email || undefined,
      mobile: applied.mobile || undefined,
      payStatus: applied.payStatus,
      isGift: applied.isGift,
      deliveryDate: applied.deliveryDate || undefined,
      mark: applied.mark !== -1 ? applied.mark : undefined,
      shippingStatus: applied.shippingStatus,
      addStartTime: applied.addTime?.[0] || undefined,
      addEndTime: applied.addTime?.[1] || undefined,
      memberTagIds: appliedMemberTagFilter.memberTagIds.length ? appliedMemberTagFilter.memberTagIds : undefined,
      memberTagMatchMode: appliedMemberTagFilter.memberTagIds.length ? appliedMemberTagFilter.memberTagMatchMode : undefined,
    })
      .then((result) => {
        if (!alive) return;
        setRows(result.records || []);
        setTotal(result.total || 0);
      })
      .catch((error) => alive && Toast.error({ content: error.message }))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [page, status, applied, appliedMemberTagFilter, reloadKey]);

  const advancedFilledCount = [
    filters.keyword,
    filters.outTradeNo,
    filters.mobile,
    filters.email,
    filters.address,
    filters.deliveryDate,
  ].filter((value) => value.trim()).length;
  const batchSnCount = /[\n,，]/.test(filters.keyword)
    ? filters.keyword
        .split(/[\n,，]+/)
        .map((part) => part.trim())
        .filter(Boolean).length
    : 0;

  const search = () => {
    setPage(1);
    setApplied({ ...filters });
  };

  const reset = () => {
    setFilters(emptyFilters);
    setApplied(emptyFilters);
    setComboField('keyword');
    setBatchSnText('');
    setPage(1);
  };

  const reload = () => setReloadKey((key) => key + 1);

  const openMemberTagDialog = () => {
    setMemberTagDialogOpen(true);
    if (tagGroups.length > 0) return;
    setTagLoading(true);
    apiGet<any>('/user/memberTagGroup/list', { page: 1, size: 100 })
      .then((result) =>
        setTagGroups(
          (result?.records || []).map((group: any) => ({
            groupId: group.groupId ?? group.id,
            groupName: group.groupName || '',
            tags: (group.tags || []).map((tag: any) => ({ id: tag.id, tagName: tag.tagName })),
          })),
        ),
      )
      .catch((error) => Toast.error({ content: error?.message || t('加载会员标签失败') }))
      .finally(() => setTagLoading(false));
  };

  const toggleMemberTag = (tagId: number) => {
    setMemberTagFilter((current) => ({
      ...current,
      memberTagIds: current.memberTagIds.includes(tagId)
        ? current.memberTagIds.filter((id) => id !== tagId)
        : [...current.memberTagIds, tagId],
    }));
  };

  const remindDeliver = (row: OrderRow) => {
    Modal.confirm({
      title: t('确认催发货该订单吗？'),
      okText: t('确定'),
      cancelText: t('取消'),
      onOk: async () => {
        try {
          await apiPost(`/order/order/remindDeliver?id=${row.orderId}`);
          Toast.success({ content: t('催发货已通知供应商') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const confirmReceipt = (row: OrderRow) => {
    const finished = row.shippingTypeId === 3;
    Modal.confirm({
      title: finished ? t('确认该订单已完成配送吗？') : t('确认订单已收货吗？'),
      okText: t('确定'),
      cancelText: t('取消'),
      onOk: async () => {
        try {
          await apiPost('/order/order/confirmReceipt', { id: row.orderId });
          Toast.success({ content: t('操作成功') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  return (
    <>
      <Card className="table-card" shadows="hover">
        <Tabs
          type="line"
          activeKey={status}
          onChange={(key) => {
            setStatus(String(key));
            setPage(1);
          }}
          tabList={statusTabs.map((item) => ({ tab: t(item.tab), itemKey: item.key }))}
        />

        {/* 搜索栏（改版）：原关键词项是多行 TextArea（批量订单号），在单行搜索栏里独占两行高；
            改版后文本检索合并为「字段下拉 + 单行输入」组合控件，批量订单号移入弹窗，
            其余筛选项统一单行高度、按容器宽度自然换行。五个文本字段的值各自保存在
            filters 中（切换字段不丢值、搜索时全部生效），高级筛选展开区可同时直填，能力不减。 */}
        <div className="orders-filter-bar">
          <Input
            className="orders-combo"
            addonBefore={
              <Select
                className="orders-combo-field"
                value={comboField}
                optionList={comboFieldDefs.map((def) => ({ label: t(def.label), value: def.field }))}
                onChange={(next) => setComboField(next as ComboField)}
              />
            }
            placeholder={t(comboFieldDefs.find((def) => def.field === comboField)?.placeholder || '')}
            value={filters[comboField]}
            onChange={(next) => setFilters((current) => ({ ...current, [comboField]: next }))}
            onEnterPress={search}
            showClear
          />
          <Button
            onClick={() => {
              setBatchSnText(filters.keyword);
              setBatchSnOpen(true);
            }}
          >
            {t('批量订单号')}
            {batchSnCount > 1 ? ` (${batchSnCount})` : ''}
          </Button>
          <Select
            className="orders-filter-select"
            placeholder={t('支付状态')}
            value={filters.payStatus}
            optionList={[
              { label: t('全部'), value: -1 },
              { label: t('未支付'), value: 0 },
              { label: t('支付中'), value: 1 },
              { label: t('已支付'), value: 2 },
              { label: t('退款中'), value: 3 },
              { label: t('已退款'), value: 4 },
            ]}
            onChange={(next) =>
              setFilters((current) => ({
                ...current,
                payStatus: Number(next) === -1 ? undefined : Number(next),
              }))
            }
          />
          <Select
            className="orders-filter-select"
            placeholder={t('订单标记')}
            value={filters.mark}
            showClear
            optionList={[
              { label: t('全部订单'), value: -1 },
              { label: <span style={{ color: '#ff5050' }}>●</span>, value: 1 },
              { label: <span style={{ color: '#ffba1f' }}>●</span>, value: 2 },
              { label: <span style={{ color: '#2ecf92' }}>●</span>, value: 3 },
              { label: <span style={{ color: '#16afff' }}>●</span>, value: 4 },
              { label: <span style={{ color: '#792dff' }}>●</span>, value: 5 },
            ]}
            onChange={(next) => setFilters((current) => ({ ...current, mark: Number(next ?? -1) }))}
          />
          <Select
            className="orders-filter-select"
            placeholder={t('送礼物')}
            value={filters.isGift}
            optionList={[
              { label: t('全部订单'), value: -1 },
              { label: t('礼物订单'), value: 1 },
              { label: t('非礼物订单'), value: 0 },
            ]}
            onChange={(next) =>
              setFilters((current) => ({
                ...current,
                isGift: Number(next) === -1 ? undefined : Number(next),
              }))
            }
          />
          <Select
            className="orders-filter-select"
            placeholder={t('发货状态')}
            value={filters.shippingStatus}
            optionList={[
              { label: t('全部'), value: -1 },
              { label: t('待发货'), value: 0 },
              { label: t('已发货'), value: 1 },
              { label: t('已收货'), value: 2 },
              { label: t('配送失败'), value: 3 },
              { label: t('配送中'), value: 4 },
              { label: t('已发货，待配送'), value: 5 },
            ]}
            onChange={(next) =>
              setFilters((current) => ({
                ...current,
                shippingStatus: Number(next) === -1 ? undefined : Number(next),
              }))
            }
          />
          <DatePicker
            className="orders-filter-date"
            type="dateRange"
            value={filters.addTime as never}
            onChange={(next) => setFilters((current) => ({ ...current, addTime: (next as string[]) || [] }))}
          />
          <Button theme="solid" type="primary" icon={<IconSearch />} onClick={search}>
            {t('搜索')}
          </Button>
          <Button onClick={reset}>{t('重置')}</Button>
          <Button onClick={openMemberTagDialog}>
            {t('按会员标签筛选')}
            {appliedMemberTagFilter.memberTagIds.length > 0 ? ` (${appliedMemberTagFilter.memberTagIds.length})` : ''}
          </Button>
          <Button theme="borderless" onClick={() => setAdvancedOpen((open) => !open)}>
            {t('高级筛选')}
            {advancedFilledCount > 0 ? ` (${advancedFilledCount})` : ''}
            <IconChevronDown
              style={{ transform: advancedOpen ? 'rotate(180deg)' : undefined, transition: 'transform .2s' }}
            />
          </Button>
        </div>
        {advancedOpen && (
          <div className="orders-filter-advanced">
            <Input
              placeholder={t('订单号 / 收货人 / 商品名称')}
              value={filters.keyword}
              onChange={(next) => setFilters((current) => ({ ...current, keyword: next }))}
              onEnterPress={search}
              showClear
            />
            <Input
              placeholder={t('交易单号')}
              value={filters.outTradeNo}
              onChange={(next) => setFilters((current) => ({ ...current, outTradeNo: next }))}
              onEnterPress={search}
              showClear
            />
            <Input
              placeholder={t('手机号')}
              value={filters.mobile}
              onChange={(next) => setFilters((current) => ({ ...current, mobile: next }))}
              onEnterPress={search}
              showClear
            />
            <Input
              placeholder="Email"
              value={filters.email}
              onChange={(next) => setFilters((current) => ({ ...current, email: next }))}
              onEnterPress={search}
              showClear
            />
            <Input
              placeholder={t('详细地址')}
              value={filters.address}
              onChange={(next) => setFilters((current) => ({ ...current, address: next }))}
              onEnterPress={search}
              showClear
            />
            {/* 送达日期筛选（我方新增，论坛帖 467744）：按买家所选送达时段备货 */}
            <DatePicker
              type="date"
              placeholder={t('送达日期')}
              value={filters.deliveryDate || undefined}
              onChange={(next) => {
                let date = '';
                if (next instanceof Date) {
                  const pad = (n: number) => String(n).padStart(2, '0');
                  date = `${next.getFullYear()}-${pad(next.getMonth() + 1)}-${pad(next.getDate())}`;
                } else if (typeof next === 'string') {
                  date = next.slice(0, 10);
                }
                setFilters((current) => ({ ...current, deliveryDate: date }));
              }}
              style={{ width: 150 }}
            />
          </div>
        )}

        <div className="orders-action-bar">
          <Button icon={<IconRefresh />} onClick={reload}>
            {t('刷新')}
          </Button>
          <Button onClick={() => setWriteOffOpen(true)}>{t('核销')}</Button>
          <Button disabled={selectedIds.length === 0} onClick={() => setBatchShipOpen(true)}>
            {t('订单批量发货')}
            {selectedIds.length > 0 ? ` (${selectedIds.length})` : ''}
          </Button>
          <Button onClick={() => setBatchImportOpen(true)}>{t('Excel批量发货')}</Button>
          <Button disabled={selectedIds.length === 0} onClick={() => setBatchSignOpen(true)}>
            {t('添加标记')}
            {selectedIds.length > 0 ? ` (${selectedIds.length})` : ''}
          </Button>
          <Button
            onClick={() => {
              if (onNavigate) onNavigate('orderExportManage');
              else window.location.href = `${window.location.origin}/?page=orderExportManage`;
            }}
          >
            {t('导出订单')}
          </Button>
        </div>

        <Table
          loading={loading}
          dataSource={rows}
          rowKey="orderId"
          scroll={{ x: 1240 }}
          rowSelection={{
            selectedRowKeys: selectedIds,
            onChange: (keys: any) => setSelectedIds((keys || []) as Array<string | number>),
          }}
          pagination={{
            currentPage: page,
            pageSize: 10,
            total,
            onChange: (nextPage: number) => setPage(nextPage),
          }}
          columns={[
            {
              title: t('订单信息'),
              dataIndex: 'orderSn',
              width: 200,
              render: (value: string, record: OrderRow) => (
                <div>
                  <div className="mono order-sn">{value}</div>
                  <div className="order-sub-text">{record.addTime || '-'}</div>
                </div>
              ),
            },
            {
              title: t('商品'),
              dataIndex: 'items',
              width: 360,
              render: (_: unknown, record: OrderRow) => (
                <OrderProductCell
                  record={record}
                  onOpen={() => {
                    setDetailId(record.orderId);
                    setDetailOpen(true);
                  }}
                />
              ),
            },
            {
              title: t('收货人'),
              dataIndex: 'consignee',
              width: 140,
              render: (value: string, record: OrderRow) => (
                <div>
                  <div>{value || '-'}</div>
                  <Text type="tertiary" size="small">
                    {record.mobile || '-'}
                  </Text>
                </div>
              ),
            },
            {
              title: t('订单金额'),
              dataIndex: 'totalAmount',
              width: 130,
              render: (value: string, record: OrderRow) => (
                <div>
                  <span className="money order-amount">¥ {Number(value || 0).toLocaleString()}</span>
                  {Number(record.paidAmount || 0) > 0 && String(record.paidAmount) !== String(value) ? (
                    <div className="order-sub-text">
                      {t('已付')} ¥ {Number(record.paidAmount).toLocaleString()}
                    </div>
                  ) : null}
                </div>
              ),
            },
            {
              title: t('状态'),
              dataIndex: 'orderStatusName',
              width: 180,
              render: (value: string, record: OrderRow) => (
                <div className="order-status-cell">
                  <Space spacing={4} wrap>
                    <Tag color={(statusColorByCode[record.orderStatus] || 'blue') as any}>{value ? t(value) : '-'}</Tag>
                    <Tag
                      color={
                        record.payStatus === 2 ? 'green' : record.payStatus === 4 ? 'grey' : 'orange'
                      }
                    >
                      {record.payStatusName ? t(record.payStatusName) : '-'}
                    </Tag>
                  </Space>
                  {record.afterSalesIng ||
                  record.paidCancelRefundStatus === 0 ||
                  record.paidCancelRefundStatus === 1 ||
                  record.paidCancelRefundStatus === 2 ||
                  record.isGiftOrder ? (
                    <Space spacing={4} wrap>
                      {record.afterSalesIng ? <Tag color="orange">{t('售后中')}</Tag> : null}
                      {record.paidCancelRefundStatus === 0 || record.paidCancelRefundStatus === 1 ? (
                        <Tag color="orange">{t('买家取消退款中')}</Tag>
                      ) : null}
                      {record.paidCancelRefundStatus === 2 ? (
                        <Tag color="grey">{t('买家取消已退款')}</Tag>
                      ) : null}
                      {record.isGiftOrder ? (
                        <Tag color="violet">
                          {t('礼物')}
                          {record.giftStatus === 1 ? `·${t('待领取')}` : record.giftStatus === 2 ? `·${t('已领取')}` : ''}
                        </Tag>
                      ) : null}
                    </Space>
                  ) : null}
                </div>
              ),
            },
            {
              // 送达时段（我方新增，论坛帖 467744）：同城单买家所选时段，商家按时段备货
              title: t('送达时段'),
              dataIndex: 'deliveryTimeText',
              width: 170,
              render: (value: string | null | undefined) =>
                value ? (
                  <span>
                    {value.slice(5)}
                    <Tag color="cyan" style={{ marginLeft: 6 }}>
                      {t('同城')}
                    </Tag>
                  </span>
                ) : (
                  '-'
                ),
            },
            {
              title: t('操作'),
              width: 240,
              fixed: 'right',
              render: (_: unknown, record: OrderRow) => (
                <Space>
                  <Button
                    theme="borderless"
                    size="small"
                    onClick={() => {
                      setDetailId(record.orderId);
                      setDetailOpen(true);
                    }}
                  >
                    {t('查看详情')}
                  </Button>
                  {record.orderStatus === 1 && (
                    <Button theme="borderless" size="small" onClick={() => remindDeliver(record)}>
                      {t('催发货')}
                    </Button>
                  )}
                  {/* 主动退款【我方新增】：已支付（2）/退款中（3）且待发货/已发货/已完成 */}
                  {(record.payStatus === 2 || record.payStatus === 3) &&
                    (record.orderStatus === 1 || record.orderStatus === 2 || record.orderStatus === 5) && (
                      <Button theme="borderless" size="small" onClick={() => setProactiveOrderId(record.orderId)}>
                        {t('主动退款')}
                      </Button>
                    )}
                  {record.orderStatus === 2 && !record.isPickup && (
                    <Button theme="borderless" size="small" onClick={() => confirmReceipt(record)}>
                      {record.shippingTypeId === 3 ? t('完成配送') : t('确认收货')}
                    </Button>
                  )}
                  {/* 取消订单 / 取消发货（旧版 ChangeStatus：isChangeOrderStatus 时展示） */}
                  {Number(record.isChangeOrderStatus) === 1 && record.availableActions?.deliver && (
                    <Button
                      theme="borderless"
                      size="small"
                      type="primary"
                      onClick={() =>
                        setChangeStatus({
                          orderId: record.orderId,
                          preOrderStatus: Number(record.preOrderStatus ?? 3),
                          title: t('取消订单'),
                        })
                      }
                    >
                      {t('取消订单')}
                    </Button>
                  )}
                  {Number(record.isChangeOrderStatus) === 1 && record.availableActions?.confirmReceipt && (
                    <Button
                      theme="borderless"
                      size="small"
                      type="primary"
                      onClick={() =>
                        setChangeStatus({
                          orderId: record.orderId,
                          preOrderStatus: Number(record.preOrderStatus ?? 1),
                          title: t('取消发货'),
                        })
                      }
                    >
                      {t('取消发货')}
                    </Button>
                  )}
                </Space>
              ),
            },
          ]}
        />
      </Card>

      <OrderDetailModal
        visible={detailOpen}
        orderId={detailId}
        onCancel={() => setDetailOpen(false)}
        onChanged={reload}
      />

      <ProactiveRefundModal
        visible={proactiveOrderId !== null}
        orderId={proactiveOrderId}
        onCancel={() => setProactiveOrderId(null)}
        onSuccess={reload}
      />

      <WriteOffModal
        visible={writeOffOpen}
        onCancel={() => setWriteOffOpen(false)}
        onSuccess={() => {
          setWriteOffOpen(false);
          reload();
        }}
      />

      <BatchSignModal
        visible={batchSignOpen}
        ids={selectedIds.map(Number)}
        onCancel={() => setBatchSignOpen(false)}
        onSuccess={() => {
          setBatchSignOpen(false);
          setSelectedIds([]);
          reload();
        }}
      />

      <BatchShipModal
        visible={batchShipOpen}
        ids={selectedIds.map(Number)}
        logistics={logistics}
        onCancel={() => setBatchShipOpen(false)}
        onSuccess={() => {
          setBatchShipOpen(false);
          setSelectedIds([]);
          reload();
        }}
      />

      <BatchDeliverImportModal
        visible={batchImportOpen}
        onCancel={() => setBatchImportOpen(false)}
        onSuccess={reload}
      />

      <ChangeStatusModal
        visible={Boolean(changeStatus)}
        orderId={changeStatus?.orderId ?? 0}
        preOrderStatus={changeStatus?.preOrderStatus ?? 3}
        title={changeStatus?.title ?? ''}
        onCancel={() => setChangeStatus(null)}
        onSuccess={() => {
          setChangeStatus(null);
          reload();
        }}
      />

      <Modal
        title={t('批量订单号')}
        visible={batchSnOpen}
        width={560}
        okText={t('搜索')}
        cancelText={t('取消')}
        onCancel={() => setBatchSnOpen(false)}
        onOk={() => {
          setFilters((current) => ({ ...current, keyword: batchSnText }));
          setApplied({ ...filters, keyword: batchSnText });
          setComboField('keyword');
          setPage(1);
          setBatchSnOpen(false);
        }}
      >
        <TextArea
          placeholder={t('粘贴多个订单号，可换行或用逗号分隔')}
          value={batchSnText}
          onChange={(next) => setBatchSnText(next)}
          autosize={{ minRows: 5, maxRows: 10 }}
        />
      </Modal>

      <Modal
        title={t('按会员标签筛选订单')}
        visible={memberTagDialogOpen}
        width={720}
        footer={
          <Space>
            <Button
              onClick={() => {
                setMemberTagFilter(emptyMemberTagFilter);
                setAppliedMemberTagFilter(emptyMemberTagFilter);
                setMemberTagDialogOpen(false);
                setPage(1);
              }}
            >
              {t('清除筛选')}
            </Button>
            <Button
              onClick={() => {
                setMemberTagDialogOpen(false);
                setPage(1);
              }}
            >
              {t('取消')}
            </Button>
            <Button
              theme="solid"
              type="primary"
              onClick={() => {
                setAppliedMemberTagFilter(memberTagFilter);
                setMemberTagDialogOpen(false);
                setPage(1);
              }}
            >
              {t('确定')}
            </Button>
          </Space>
        }
      >
        <Spin spinning={tagLoading}>
          <RadioGroup
            value={memberTagFilter.memberTagMatchMode}
            onChange={(event) =>
              setMemberTagFilter((current) => ({
                ...current,
                memberTagMatchMode: Number(event.target.value),
              }))
            }
            style={{ marginBottom: 12 }}
          >
            <Radio value={0}>{t('匹配任一标签')}</Radio>
            <Radio value={1}>{t('匹配全部标签')}</Radio>
          </RadioGroup>
          <div style={{ maxHeight: 380, overflowY: 'auto' }}>
            {tagGroups.map((group) => (
              <div key={group.groupId} style={{ marginBottom: 12 }}>
                <div style={{ fontWeight: 500, marginBottom: 6 }}>{group.groupName}</div>
                <Space wrap>
                  {group.tags.map((tag) => (
                    <Tag
                      key={tag.id}
                      color={memberTagFilter.memberTagIds.includes(tag.id) ? 'blue' : 'white'}
                      style={{ cursor: 'pointer' }}
                      onClick={() => toggleMemberTag(tag.id)}
                    >
                      {tag.tagName}
                    </Tag>
                  ))}
                </Space>
              </div>
            ))}
            {!tagLoading && tagGroups.length === 0 && <Empty description={t('暂无标签')} />}
          </div>
        </Spin>
      </Modal>
    </>
  );
}
