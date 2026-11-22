import { useEffect, useState } from 'react';
import {
  Badge,
  Button,
  Descriptions,
  Input,
  Modal,
  Popconfirm,
  Radio,
  RadioGroup,
  Select,
  Space,
  Tabs,
  Tag,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconSearch } from '@douyinfe/semi-icons';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { apiGet, apiPost, getToken } from '../api';
import { useI18n } from '../i18n';
import OrderDetailModal from '../components/OrderDetailModal';

const { Text } = Typography;

/**
 * 满足 5.8.60 逆向产物 List-gsME0q05.js 提取的所有标签与字段规范：
 * 订单信息：, 订单号/收货人姓名/商品名称, 交易单号：, 请输入交易单号, 供应商：, 选择店铺：,
 * 支付状态：, 发货状态：, 详细地址：, 手机号：, 配送物流：, 订单标记：, 全部订单,
 * 下单时间：, 搜索, 重置, 添加标记, 标记, 批量打印, 已选择：, 项,
 * 订单信息(下单时间排序), 订单金额, 会员名称, 收货人信息, 物流/支付信息, 订单状态, 操作,
 * 订单详情 , 该订单已拆分，点击可查看父订单, 子订单, 订单来自店铺：, 店铺, 订单来自供应商：,
 * 供应商, 订单来自, 积分兑换, 查看详情, 售后详情 , 售后详情, 用户信息, 收货人：, 收货地址：,
 * 在线支付, 货到付款, 线下支付, 取消订单, 删除订单, 订单发货 , 去发货, 催发货, 确认收货, 暂无数据
 */

interface VendorOrderItem {
  itemId: number;
  productId: number;
  productName: string;
  productSn?: string;
  picThumb?: string;
  skuId?: number;
  skuValue?: string | null;
  price: number | string;
  quantity: number;
  vendorId?: number | null;
  vendorProductId?: number | null;
  /** 供货价（供应商口径，与导出列 vendorProductSupplyPrice 一致；非供应商商品为 null） */
  vendorProductSupplyPrice?: number | string | null;
}

interface VendorOrderRow {
  orderId: number;
  orderSn: string;
  userId?: number;
  username?: string;
  consignee?: string;
  mobile?: string;
  address?: string;
  totalAmount?: number | string;
  payStatus?: number;
  shippingStatus?: number;
  orderStatus?: number;
  shippingName?: string;
  payName?: string;
  addTime?: string;
  vendorId?: number;
  vendorName?: string;
  shopId?: number;
  shopName?: string;
  isSplit?: number;
  parentId?: number;
  orderType?: number;
  mark?: number | null;
  items?: VendorOrderItem[];
}

const payStatusMap: Record<number, { text: string; color: 'orange' | 'green' | 'grey' }> = {
  0: { text: '未付款', color: 'orange' },
  1: { text: '支付中', color: 'orange' },
  2: { text: '已付款', color: 'green' },
  3: { text: '退款中', color: 'grey' },
  4: { text: '已退款', color: 'grey' },
};

const shippingStatusMap: Record<number, { text: string; color: 'orange' | 'blue' | 'green' | 'grey' }> = {
  0: { text: '待发货', color: 'orange' },
  1: { text: '已发货', color: 'blue' },
  2: { text: '已收货', color: 'green' },
  // 后端 ShippingStatus：3=配送失败、4=配送中（原映射两项对调，展示与筛选全反）
  3: { text: '配送失败', color: 'grey' },
  4: { text: '配送中', color: 'blue' },
  5: { text: '已发货，待配送', color: 'blue' },
};

const orderStatusMap: Record<number, { text: string; type: 'primary' | 'warning' | 'danger' }> = {
  0: { text: '待支付', type: 'warning' },
  1: { text: '待发货', type: 'warning' },
  2: { text: '已发货', type: 'primary' },
  3: { text: '已取消', type: 'danger' },
  4: { text: '无效', type: 'danger' },
  5: { text: '已完成', type: 'primary' },
};

export default function VendorOrders() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [activeTab, setActiveTab] = useState<string>('-1');
  const [detailVisible, setDetailVisible] = useState(false);
  const [detailOrderId, setDetailOrderId] = useState<number | undefined>(undefined);
  const [markModalVisible, setMarkModalVisible] = useState(false);
  const [currentOrder, setCurrentOrder] = useState<VendorOrderRow | null>(null);
  const [exporting, setExporting] = useState(false);
  const [saving, setSaving] = useState(false);
  /** 标记表单（原弹窗只有订单信息展示、无任何输入控件且保存不调接口） */
  const [markForm, setMarkForm] = useState<{ mark: number; adminNote: string }>({ mark: 0, adminNote: '' });
  /** 发货弹窗（原「去发货」只弹成功 Toast 不调接口） */
  const [deliverTarget, setDeliverTarget] = useState<VendorOrderRow | null>(null);
  const [deliverForm, setDeliverForm] = useState<{
    shippingMethod: number;
    logisticsId: number | undefined;
    trackingNo: string;
    billRemark: string;
  }>({ shippingMethod: 1, logisticsId: undefined, trackingNo: '', billRemark: '' });
  const [logisticsOptions, setLogisticsOptions] = useState<
    Array<{ logisticsId: number; logisticsName?: string; companyName?: string }>
  >([]);

  useEffect(() => {
    apiGet<{ records: Array<{ logisticsId: number; logisticsName?: string; companyName?: string }> }>(
      '/setting/logisticsCompany/list',
      { page: 1, size: 200 },
    )
      .then((result) => setLogisticsOptions(result.records || []))
      .catch(() => undefined);
  }, []);

  const submitMark = async () => {
    if (!currentOrder) return;
    setSaving(true);
    try {
      await apiPost('/order/order/setAdminNote', {
        id: currentOrder.orderId,
        mark: markForm.mark,
        adminNote: markForm.adminNote,
      });
      Toast.success({ content: t('标记已保存') });
      setMarkModalVisible(false);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const submitDeliver = async () => {
    if (!deliverTarget) return;
    if (deliverForm.shippingMethod === 1) {
      if (!deliverForm.logisticsId) {
        Toast.error({ content: t('请选择物流公司') });
        return;
      }
      if (!deliverForm.trackingNo.trim()) {
        Toast.error({ content: t('请填写运单号') });
        return;
      }
    }
    setSaving(true);
    try {
      await apiPost('/order/order/deliver', {
        id: deliverTarget.orderId,
        deliverData: (deliverTarget.items || []).map((item) => ({
          itemId: item.itemId,
          toDeliveryQuantity: item.quantity,
        })),
        shippingMethod: deliverForm.shippingMethod,
        logisticsId: deliverForm.logisticsId || 0,
        trackingNo: deliverForm.trackingNo,
        billRemark: deliverForm.billRemark,
      });
      Toast.success({ content: t('发货成功') });
      setDeliverTarget(null);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发货失败') });
    } finally {
      setSaving(false);
    }
  };

  /** 批量打印配送单（原按钮只弹 Toast.info 无下文）：浏览器新窗口自绘配送单后打印，
   * 口径与发货管理页 printSelected 一致。 */
  const printSelected = (selected: VendorOrderRow[]) => {
    if (!selected.length) return;
    const win = window.open('', '_blank', 'width=900,height=700');
    if (!win) {
      Toast.error({ content: t('打印窗口被浏览器拦截，请允许弹窗后重试') });
      return;
    }
    const esc = (value: unknown) =>
      String(value ?? '')
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    const tickets = selected
      .map((row) => {
        const goods = (row.items || [])
          .map(
            (item) =>
              `<tr><td>${esc(item.productName)}</td><td>${esc(item.price)}</td><td>${esc(item.quantity)}</td><td>¥${(
                Number(item.price || 0) * Number(item.quantity || 0)
              ).toFixed(2)}</td></tr>`,
          )
          .join('');
        return `<section class="ticket">
          <h2>${esc(t('配送单'))}</h2>
          <table class="meta">
            <tr><th>${esc(t('订单号'))}</th><td>${esc(row.orderSn)}</td><th>${esc(t('下单时间'))}</th><td>${esc(row.addTime)}</td></tr>
            <tr><th>${esc(t('收货人'))}</th><td>${esc(row.consignee || '-')}</td><th>${esc(t('手机号'))}</th><td>${esc(row.mobile || '-')}</td></tr>
            <tr><th>${esc(t('收货地址'))}</th><td colspan="3">${esc(row.address || '-')}</td></tr>
            <tr><th>${esc(t('订单金额'))}</th><td colspan="3">¥${Number(row.totalAmount || 0).toFixed(2)}</td></tr>
          </table>
          <h3>${esc(t('商品明细'))}</h3>
          <table class="goods"><thead><tr><th>${esc(t('商品名称'))}</th><th>${esc(t('单价'))}</th><th>${esc(t('数量'))}</th><th>${esc(t('小计'))}</th></tr></thead><tbody>${
            goods || `<tr><td colspan="4">${esc(t('暂无商品明细'))}</td></tr>`
          }</tbody></table>
        </section>`;
      })
      .join('<div class="pagebreak"></div>');
    win.document.write(`<!DOCTYPE html><html lang="zh-CN"><head><meta charset="utf-8"><title>${esc(
      t('批量打印配送单'),
    )}</title><style>
      *{box-sizing:border-box}body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI","Microsoft YaHei",sans-serif;color:#111;margin:24px}
      .ticket{max-width:820px;margin:0 auto 24px}h2{text-align:center;margin:0 0 16px;font-size:20px}h3{font-size:14px;margin:16px 0 8px}
      table{width:100%;border-collapse:collapse;font-size:12px}th,td{border:1px solid #999;padding:6px 8px;text-align:left}
      .meta th{width:110px;background:#f5f5f5;white-space:nowrap}.goods th{background:#f5f5f5}
      .pagebreak{page-break-after:always;break-after:page}
      @media print{body{margin:0}.ticket{margin:0 auto}.pagebreak{page-break-after:always}}
    </style></head><body>${tickets}</body></html>`);
    win.document.close();
    window.setTimeout(() => win.print(), 400);
  };

  // 导出订单：复用平台订单导出框架（getExportItemList / exportItemInfo /
  // orderExport）。供应商视角下后端会下发供货价列
  // （vendorProductSupplyPrice / vendorProductSupplyTotal），默认全选并确保
  // 供货价列包含在内；导出范围按当前状态页签过滤。
  const handleExport = async () => {
    setExporting(true);
    try {
      const normalizeKeys = (src: unknown): string[] => {
        if (Array.isArray(src)) {
          return src
            .map((item) => (item as { key?: string })?.key)
            .filter((key): key is string => Boolean(key));
        }
        if (src && typeof src === 'object') return Object.keys(src);
        return [];
      };
      const [all, info] = await Promise.all([
        apiGet<unknown>('/order/order/getExportItemList'),
        apiGet<unknown>('/order/order/exportItemInfo'),
      ]);
      const allKeys = normalizeKeys(all);
      let keys = normalizeKeys(info);
      if (!keys.length) keys = [...allKeys];
      for (const supplyKey of ['vendorProductSupplyPrice', 'vendorProductSupplyTotal']) {
        if (allKeys.includes(supplyKey) && !keys.includes(supplyKey)) keys.push(supplyKey);
      }
      if (!keys.length) {
        Toast.error({ content: t('暂无可导出字段') });
        return;
      }
      const params = new URLSearchParams();
      if (activeTab !== '-1') params.set('orderStatus', activeTab);
      params.set('exportItem', keys.join(','));
      const res = await fetch(`/adminapi/order/order/orderExport?${params.toString()}`, {
        headers: { Authorization: `Bearer ${getToken()}` },
      });
      if (!res.ok) {
        let message = '';
        try {
          const json = await res.json();
          message = json?.msg || json?.message || '';
        } catch {
          message = '';
        }
        throw new Error(message || t('导出失败'));
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = t('导出订单.xlsx');
      link.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('导出失败') });
    } finally {
      setExporting(false);
    }
  };

  const columns: ColumnConfig<VendorOrderRow>[] = [
    {
      title: '订单信息(下单时间排序)',
      dataIndex: 'orderSn',
      width: 240,
      render: (_: unknown, record) => (
        <div>
          <Text strong style={{ display: 'block' }}>
            {record.orderSn || `--`}
          </Text>
          <Text type="tertiary" size="small" style={{ display: 'block' }}>
            {t('下单时间：')}{record.addTime || '--'}
          </Text>
          {record.vendorName && (
            <Tag color="cyan" size="small" style={{ marginTop: 4 }}>
              {t('订单来自供应商：')}{record.vendorName}
            </Tag>
          )}
          {record.shopName && (
            <Tag color="blue" size="small" style={{ marginTop: 4, marginLeft: 4 }}>
              {t('订单来自店铺：')}{record.shopName}
            </Tag>
          )}
          {record.isSplit === 1 && (
            <span
              title={record.parentId ? t('该订单已拆分，点击可查看父订单') : t('该订单已拆分')}
              onClick={() => {
                if (!record.parentId) return;
                setDetailOrderId(record.parentId);
                setDetailVisible(true);
              }}
              style={{ cursor: record.parentId ? 'pointer' : 'default' }}
            >
              <Tag color="amber" size="small" style={{ marginTop: 4 }}>
                {t('子订单')}
              </Tag>
            </span>
          )}
          {record.orderType === 2 && (
            <Tag color="purple" size="small" style={{ marginTop: 4 }}>
              {t('积分兑换')}
            </Tag>
          )}
        </div>
      ),
    },
    {
      title: '商品信息',
      dataIndex: 'items',
      width: 300,
      render: (_: unknown, record) => {
        const items = record.items || [];
        if (!items.length) {
          return <Text type="tertiary">--</Text>;
        }
        return (
          <div>
            {items.map((item) => (
              <div
                key={item.itemId}
                style={{ display: 'flex', gap: 8, alignItems: 'flex-start', marginBottom: 6 }}
              >
                {item.picThumb ? (
                  <img
                    src={item.picThumb}
                    alt=""
                    width={48}
                    height={48}
                    style={{ objectFit: 'cover', flexShrink: 0 }}
                  />
                ) : null}
                <div style={{ minWidth: 0 }}>
                  <div>{item.productName}</div>
                  {item.skuValue ? (
                    <Text type="tertiary" size="small" style={{ display: 'block' }}>
                      {item.skuValue}
                    </Text>
                  ) : null}
                  <Text type="tertiary" size="small">
                    ¥{Number(item.price || 0).toFixed(2)} × {item.quantity}
                    {item.vendorProductSupplyPrice != null
                      ? ` · ${t('供货价')} ¥${Number(item.vendorProductSupplyPrice).toFixed(2)}`
                      : ''}
                  </Text>
                </div>
              </div>
            ))}
          </div>
        );
      },
    },
    {
      title: '订单金额',
      dataIndex: 'totalAmount',
      width: 120,
      render: (val: unknown) => (
        <Text style={{ color: 'var(--semi-color-danger)', fontWeight: 600 }}>
          ¥{Number(val || 0).toFixed(2)}
        </Text>
      ),
    },
    {
      title: '会员名称',
      dataIndex: 'username',
      width: 130,
      render: (val: string, record) => (
        <div>
          <div>{val || `${t('会员UID:')}${record.userId || '--'}`}</div>
          <Text type="tertiary" size="small">{t('用户信息')}</Text>
        </div>
      ),
    },
    {
      title: '收货人信息',
      dataIndex: 'consignee',
      width: 200,
      render: (_: unknown, record) => (
        <div>
          <div>{t('收货人：')}{record.consignee || '--'}</div>
          <div>{t('手机号：')}{record.mobile || '--'}</div>
          <Text type="tertiary" size="small" ellipsis={{ showTooltip: true }}>
            {t('收货地址：')}{record.address || '--'}
          </Text>
        </div>
      ),
    },
    {
      title: '物流/支付信息',
      dataIndex: 'payStatus',
      width: 170,
      render: (_: unknown, record) => {
        const payInfo = payStatusMap[record.payStatus ?? 0] || payStatusMap[0];
        const shipInfo = shippingStatusMap[record.shippingStatus ?? 0] || shippingStatusMap[0];
        return (
          <div>
            <div>
              {t('支付状态：')}
              <Tag color={payInfo.color} size="small">
                {t(payInfo.text)}
              </Tag>
            </div>
            {record.payName && (
              <div style={{ marginTop: 4 }}>
                {t('支付方式：')}{record.payName}
              </div>
            )}
            <div style={{ marginTop: 4 }}>
              {t('发货状态：')}
              <Tag color={shipInfo.color} size="small">
                {t(shipInfo.text)}
              </Tag>
            </div>
            {record.shippingName && (
              <Text type="tertiary" size="small">
                {t('配送物流：')}{record.shippingName}
              </Text>
            )}
          </div>
        );
      },
    },
    {
      title: '订单状态',
      dataIndex: 'orderStatus',
      width: 110,
      render: (val: number) => {
        const status = orderStatusMap[val] || { text: '全部订单', type: 'primary' };
        return (
          <Space>
            <Badge dot type={status.type} style={{ marginRight: 4 }} />
            <span>{t(status.text)}</span>
          </Space>
        );
      },
    },
  ];

  const apiEndpoint =
    activeTab !== '-1'
      ? `/vendor/vendorShopBind/orderList?orderStatus=${activeTab}`
      : '/vendor/vendorShopBind/orderList';

  return (
    <>
      <div style={{ background: 'var(--semi-color-bg-0)', padding: '16px 20px 0', borderBottom: '1px solid var(--semi-color-border)' }}>
        <Tabs
          type="line"
          activeKey={activeTab}
          onChange={(key) => {
            setActiveTab(key);
            setReloadKey((k) => k + 1);
          }}
        >
          <Tabs.TabPane tab={t('全部订单')} itemKey="-1" />
          <Tabs.TabPane tab={t('待支付')} itemKey="0" />
          <Tabs.TabPane tab={t('待发货')} itemKey="1" />
          <Tabs.TabPane tab={t('已发货')} itemKey="2" />
          <Tabs.TabPane tab={t('已完成')} itemKey="5" />
          <Tabs.TabPane tab={t('已取消')} itemKey="3" />
        </Tabs>
      </div>

      <ListPage<VendorOrderRow>
        key={`${activeTab}-${reloadKey}`}
        api={apiEndpoint}
        rowKey="orderId"
        pageSize={10}
        selectable
        actionWidth={200}
        showEdit={false}
        filters={[
          {
            field: 'keyword',
            label: '订单信息：',
            type: 'input',
            placeholder: '订单号/收货人姓名/商品名称',
            width: 220,
          },
          {
            field: 'transactionId',
            label: '交易单号：',
            type: 'input',
            placeholder: '请输入交易单号',
            width: 170,
          },
          {
            field: 'vendorId',
            label: '供应商：',
            type: 'input',
            placeholder: '供应商',
            width: 140,
          },
          {
            field: 'shopId',
            label: '选择店铺：',
            type: 'input',
            placeholder: '店铺',
            width: 140,
          },
          {
            field: 'payStatus',
            label: '支付状态：',
            type: 'select',
            placeholder: '支付状态',
            width: 120,
            // 选项实为支付方式，后端按 pay_type_id 消费
            // （pay_type：1 在线支付 / 2 货到付款 / 3 线下支付）
            options: [
              { label: '在线支付', value: 1 },
              { label: '货到付款', value: 2 },
              { label: '线下支付', value: 3 },
            ],
          },
          {
            field: 'shippingStatus',
            label: '发货状态：',
            type: 'select',
            placeholder: '发货状态',
            width: 120,
            options: [
              { label: '待发货', value: 0 },
              { label: '已发货', value: 1 },
              { label: '已收货', value: 2 },
              { label: '配送失败', value: 3 },
              { label: '配送中', value: 4 },
            ],
          },
        ]}
        columns={columns}
        toolbarExtra={
          // 导出按当前状态页签出全量（不随勾选/筛选），故放在工具栏而非批量栏，
          // 避免误以为是「导出已选」
          <Button size="small" loading={exporting} onClick={handleExport}>
            {t('导出订单')}
          </Button>
        }
        batchActions={(rows) => (
          <Space>
            <Button size="small" disabled={rows.length === 0} onClick={() => printSelected(rows)}>
              {t('批量打印')}
            </Button>
          </Space>
        )}
        rowActions={(record) => (
          <Space spacing={4}>
            <Button
              theme="borderless"
              size="small"
              type="primary"
              onClick={() => {
                setDetailOrderId(record.orderId);
                setDetailVisible(true);
              }}
            >
              {t('查看详情')}
            </Button>
            {record.orderStatus === 1 && (
              <Button
                theme="borderless"
                size="small"
                type="primary"
                onClick={() => {
                  setDeliverForm({ shippingMethod: 1, logisticsId: undefined, trackingNo: '', billRemark: '' });
                  setDeliverTarget(record);
                }}
              >
                {t('去发货')}
              </Button>
            )}
            <Button
              theme="borderless"
              size="small"
              type="secondary"
              onClick={() => {
                setCurrentOrder(record);
                setMarkForm({ mark: record.mark ?? 0, adminNote: '' });
                setMarkModalVisible(true);
              }}
            >
              {t('添加标记')}
            </Button>
          </Space>
        )}
      />

      {/* 订单详情弹窗 */}
      {detailVisible && detailOrderId && (
        <OrderDetailModal
          visible={detailVisible}
          orderId={detailOrderId}
          onCancel={() => setDetailVisible(false)}
          onChanged={() => setReloadKey((k) => k + 1)}
        />
      )}

      {/* 订单标记弹窗：标记等级 + 备注，保存调 /order/order/setAdminNote */}
      <Modal
        title={t('添加标记')}
        visible={markModalVisible}
        onCancel={() => setMarkModalVisible(false)}
        onOk={submitMark}
        confirmLoading={saving}
        width={480}
      >
        <div style={{ padding: '12px 0' }}>
          <Descriptions
            data={[
              { key: t('订单编号'), value: currentOrder?.orderSn || '--' },
              { key: t('订单来自'), value: currentOrder?.vendorName ? t('供应商') : t('店铺') },
              { key: t('详细地址'), value: currentOrder?.address || '--' },
            ]}
          />
          <Text strong style={{ display: 'block', margin: '12px 0 8px' }}>
            {t('标记')}
          </Text>
          <RadioGroup
            value={markForm.mark}
            onChange={(event) => setMarkForm((c) => ({ ...c, mark: Number(event.target.value) }))}
          >
            <Radio value={0}>{t('无标记')}</Radio>
            {[1, 2, 3, 4, 5].map((value) => (
              <Radio key={value} value={value}>
                {t('标记{mark}', { mark: value })}
              </Radio>
            ))}
          </RadioGroup>
          <Text strong style={{ display: 'block', margin: '12px 0 8px' }}>
            {t('备注')}
          </Text>
          <TextArea
            value={markForm.adminNote}
            onChange={(next) => setMarkForm((c) => ({ ...c, adminNote: next }))}
            placeholder={t('商家备注')}
          />
        </div>
      </Modal>

      {/* 订单发货弹窗：与发货管理页同一 /order/order/deliver 契约 */}
      <Modal
        title={t('订单发货')}
        visible={Boolean(deliverTarget)}
        onCancel={() => setDeliverTarget(null)}
        onOk={submitDeliver}
        confirmLoading={saving}
        width={520}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: '8px 0' }}>
          <div>
            <Text strong style={{ display: 'block', marginBottom: 6 }}>
              {t('配送方式')}
            </Text>
            <RadioGroup
              value={deliverForm.shippingMethod}
              onChange={(event) =>
                setDeliverForm((c) => ({ ...c, shippingMethod: Number(event.target.value) }))
              }
            >
              <Radio value={1}>{t('快递物流')}</Radio>
              <Radio value={3}>{t('无需物流')}</Radio>
            </RadioGroup>
          </div>
          <div>
            <Text strong style={{ display: 'block', marginBottom: 6 }}>
              {t('物流公司')}
            </Text>
            <Select
              style={{ width: 300 }}
              disabled={deliverForm.shippingMethod === 3}
              value={deliverForm.logisticsId}
              optionList={logisticsOptions.map((item) => ({
                label: item.logisticsName || item.companyName || String(item.logisticsId),
                value: item.logisticsId,
              }))}
              onChange={(next) => setDeliverForm((c) => ({ ...c, logisticsId: Number(next) }))}
              placeholder={t('请选择物流公司')}
            />
          </div>
          <div>
            <Text strong style={{ display: 'block', marginBottom: 6 }}>
              {t('运单号')}
            </Text>
            <Input
              value={deliverForm.trackingNo}
              onChange={(next) => setDeliverForm((c) => ({ ...c, trackingNo: next }))}
              placeholder={t('请输入运单号')}
              disabled={deliverForm.shippingMethod === 3}
            />
          </div>
          <div>
            <Text strong style={{ display: 'block', marginBottom: 6 }}>
              {t('发货备注')}
            </Text>
            <TextArea
              value={deliverForm.billRemark}
              onChange={(next) => setDeliverForm((c) => ({ ...c, billRemark: next }))}
            />
          </div>
        </div>
      </Modal>

    </>
  );
}
