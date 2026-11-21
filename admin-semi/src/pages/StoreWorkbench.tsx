import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Empty,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Spin,
  Switch,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconRefresh, IconSearch } from '@douyinfe/semi-icons';
import { apiGet, apiPost, isDemo } from '../api';
import { useI18n } from '../i18n';
import type { OrderRow, PageResult, ProductRow } from '../types';
import OrderDetailModal from '../components/OrderDetailModal';

const { Text } = Typography;

/**
 * 门店工作台（我方新增，论坛帖 597800）：
 * 门店员工移动端操作台——把商家 Web 后台的高频动作（订单处理、自提核销、
 * 商品/库存速览）聚合为一页单列卡片流，大按钮适配手机操作。
 * 全部复用现有管理端接口（/order/order/*、/product/product/*），
 * 数据作用域由后端按登录令牌的店铺作用域强制限定，不新造权限体系。
 */

type StatusCounts = {
  allCount?: number;
  pendingCount?: number;
  confirmedCount?: number;
  processingCount?: number;
  cancelledCount?: number;
  completedCount?: number;
};

const ORDER_TABS: Array<{ key: string; label: string }> = [
  { key: '1', label: '待发货' },
  { key: '2', label: '待收货' },
  { key: '5', label: '已完成' },
  { key: '', label: '全部' },
];

type TagColor = 'orange' | 'blue' | 'cyan' | 'green' | 'grey' | 'violet' | 'teal';

const statusColor: Record<string, TagColor> = {
  待付款: 'orange',
  待发货: 'blue',
  待收货: 'cyan',
  已完成: 'green',
  已取消: 'grey',
};

type DetailItem = {
  itemId: number;
  productName?: string;
  quantity: number;
  deliveryQuantity?: number;
};

const PAGE_SIZE = 10;
/** 库存预警阈值（速览高亮用，与库存预警页默认口径一致的经验值）。 */
const LOW_STOCK = 10;

export default function StoreWorkbench() {
  const { t } = useI18n();
  const shopTitle = localStorage.getItem('shopTitle') || '';

  const [counts, setCounts] = useState<StatusCounts>({});
  const [tab, setTab] = useState('1');
  const [orders, setOrders] = useState<OrderRow[]>([]);
  const [orderTotal, setOrderTotal] = useState(0);
  const [orderPage, setOrderPage] = useState(1);
  const [ordersLoading, setOrdersLoading] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);

  const [pickupSn, setPickupSn] = useState('');
  const [writeOffLoading, setWriteOffLoading] = useState(false);

  const [detailId, setDetailId] = useState<number | undefined>(undefined);

  const [deliverTarget, setDeliverTarget] = useState<OrderRow | null>(null);
  const [deliverItems, setDeliverItems] = useState<DetailItem[]>([]);
  const [logistics, setLogistics] = useState<Array<{ id: number; name: string }>>([]);
  const [deliverForm, setDeliverForm] = useState<{ logisticsId?: number; trackingNo: string }>({
    trackingNo: '',
  });
  const [deliverSaving, setDeliverSaving] = useState(false);

  const [productKeyword, setProductKeyword] = useState('');
  const [productQuery, setProductQuery] = useState('');
  const [products, setProducts] = useState<ProductRow[]>([]);
  const [productTotal, setProductTotal] = useState(0);
  const [productPage, setProductPage] = useState(1);
  const [productsLoading, setProductsLoading] = useState(false);
  const [salePendingId, setSalePendingId] = useState<number | undefined>(undefined);
  const [shelfPendingId, setShelfPendingId] = useState<number | undefined>(undefined);
  const [stockTarget, setStockTarget] = useState<ProductRow | null>(null);
  const [stockValue, setStockValue] = useState<number>(0);
  const [stockSaving, setStockSaving] = useState(false);

  const reloadAll = useCallback(() => setReloadKey((key) => key + 1), []);

  /* ---------- 概览计数 ---------- */
  useEffect(() => {
    if (isDemo()) return;
    let alive = true;
    apiGet<StatusCounts>('/order/order/statusCount', {})
      .then((result) => alive && setCounts(result || {}))
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [reloadKey]);

  /* ---------- 订单流 ---------- */
  useEffect(() => {
    if (isDemo()) return;
    let alive = true;
    setOrdersLoading(true);
    apiGet<PageResult<OrderRow>>('/order/order/list', {
      page: orderPage,
      size: PAGE_SIZE,
      orderStatus: tab || undefined,
    })
      .then((result) => {
        if (!alive) return;
        setOrders((prev) =>
          orderPage === 1 ? result.records || [] : [...prev, ...(result.records || [])],
        );
        setOrderTotal(result.total || 0);
      })
      .catch((error) => alive && Toast.error({ content: error.message }))
      .finally(() => alive && setOrdersLoading(false));
    return () => {
      alive = false;
    };
  }, [tab, orderPage, reloadKey]);

  const switchTab = (key: string) => {
    setTab(key);
    setOrderPage(1);
    setOrders([]);
  };

  /* ---------- 核销 ---------- */
  const submitWriteOff = async () => {
    const code = pickupSn.trim();
    if (!code) {
      Toast.warning({ content: t('请输入核销码') });
      return;
    }
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    setWriteOffLoading(true);
    try {
      await apiPost(`/order/order/writeOff?pickupSn=${encodeURIComponent(code)}`);
      Toast.success({ content: t('核销成功') });
      setPickupSn('');
      reloadAll();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('核销失败') });
    } finally {
      setWriteOffLoading(false);
    }
  };

  /* ---------- 发货 ---------- */
  useEffect(() => {
    if (isDemo() || logistics.length > 0) return;
    apiGet<{ records: Array<{ logisticsId: number; logisticsName: string }> }>(
      '/setting/logisticsCompany/list',
      { page: 1, size: 100 },
    )
      .then((result) =>
        setLogistics(
          (result.records || []).map((item) => ({ id: item.logisticsId, name: item.logisticsName })),
        ),
      )
      .catch(() => undefined);
  }, [logistics.length]);

  const openDeliver = async (row: OrderRow) => {
    setDeliverTarget(row);
    setDeliverForm({ logisticsId: undefined, trackingNo: '' });
    setDeliverItems([]);
    try {
      const detail = await apiGet<{ items?: DetailItem[] }>('/order/order/detail', {
        id: row.orderId,
      });
      setDeliverItems(detail.items || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载订单失败') });
    }
  };

  const submitDeliver = async () => {
    if (!deliverTarget) return;
    const deliverData = deliverItems
      .map((item) => ({
        itemId: item.itemId,
        toDeliveryQuantity: Math.max(0, Number(item.quantity) - Number(item.deliveryQuantity || 0)),
      }))
      .filter((item) => item.toDeliveryQuantity > 0);
    if (deliverData.length === 0) {
      Toast.warning({ content: t('没有待发货的商品') });
      return;
    }
    setDeliverSaving(true);
    try {
      await apiPost('/order/order/deliver', {
        id: deliverTarget.orderId,
        shippingMethod: 1,
        logisticsId: deliverForm.logisticsId || 0,
        trackingNo: deliverForm.trackingNo,
        deliverData,
      });
      Toast.success({ content: t('发货成功') });
      setDeliverTarget(null);
      reloadAll();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发货失败') });
    } finally {
      setDeliverSaving(false);
    }
  };

  /* ---------- 确认收货 / 完成配送 ---------- */
  const confirmReceipt = (row: OrderRow) => {
    const label = row.shippingTypeId === 3 ? t('完成配送') : t('确认收货');
    Modal.confirm({
      title: `${label}？`,
      content: row.orderSn,
      okText: t('确定'),
      cancelText: t('取消'),
      onOk: async () => {
        try {
          await apiPost('/order/order/confirmReceipt', { id: row.orderId });
          Toast.success({ content: t('操作成功') });
          reloadAll();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  /* ---------- 商品速览 ---------- */
  useEffect(() => {
    if (isDemo()) return;
    let alive = true;
    setProductsLoading(true);
    apiGet<PageResult<ProductRow>>('/product/product/list', {
      page: productPage,
      pageSize: PAGE_SIZE,
      keyword: productQuery || undefined,
      isDelete: 0,
    })
      .then((result) => {
        if (!alive) return;
        setProducts((prev) =>
          productPage === 1 ? result.records || [] : [...prev, ...(result.records || [])],
        );
        setProductTotal(result.total || 0);
      })
      .catch((error) => alive && Toast.error({ content: error.message }))
      .finally(() => alive && setProductsLoading(false));
    return () => {
      alive = false;
    };
  }, [productPage, productQuery, reloadKey]);

  const changeSaleStatus = async (row: ProductRow, val: number) => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    setSalePendingId(row.productId);
    try {
      await apiPost('/product/product/saleStatus', {
        productId: row.productId,
        saleStatus: val,
        saleBanTime: 0,
      });
      Toast.success({ content: val === 1 ? t('商品已设为可售') : t('商品已设为禁售') });
      setProducts((prev) =>
        prev.map((item) => (item.productId === row.productId ? { ...item, saleStatus: val } : item)),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    } finally {
      setSalePendingId(undefined);
    }
  };

  const toggleShelf = async (row: ProductRow) => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    const next = row.productStatus === 1 ? 0 : 1;
    setShelfPendingId(row.productId);
    try {
      await apiPost('/product/product/updateField', {
        id: row.productId,
        field: 'productStatus',
        val: next,
      });
      Toast.success({ content: next === 1 ? t('商品已上架') : t('商品已下架') });
      setProducts((prev) =>
        prev.map((item) =>
          item.productId === row.productId ? { ...item, productStatus: next } : item,
        ),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    } finally {
      setShelfPendingId(undefined);
    }
  };

  const submitStock = async () => {
    if (!stockTarget) return;
    setStockSaving(true);
    try {
      await apiPost('/product/product/updateField', {
        id: stockTarget.productId,
        field: 'productStock',
        val: Number(stockValue || 0),
      });
      Toast.success({ content: t('库存已更新') });
      setProducts((prev) =>
        prev.map((item) =>
          item.productId === stockTarget.productId
            ? { ...item, productStock: Number(stockValue || 0) }
            : item,
        ),
      );
      setStockTarget(null);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setStockSaving(false);
    }
  };

  const statItems: Array<{ label: string; value: number; tabKey: string }> = [
    { label: '待发货', value: counts.confirmedCount || 0, tabKey: '1' },
    { label: '待收货', value: counts.processingCount || 0, tabKey: '2' },
    { label: '已完成', value: counts.completedCount || 0, tabKey: '5' },
    { label: '全部订单', value: counts.allCount || 0, tabKey: '' },
  ];

  return (
    <div style={{ maxWidth: 760, margin: '0 auto', padding: '12px 4px 32px' }}>
      {/* ===== 概览 ===== */}
      <Card style={{ marginBottom: 12 }} bodyStyle={{ padding: 16 }}>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: 12,
          }}
        >
          <Text strong style={{ fontSize: 16 }}>
            {shopTitle || t('门店工作台')}
          </Text>
          <Button icon={<IconRefresh />} theme="borderless" onClick={reloadAll}>
            {t('刷新')}
          </Button>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 8 }}>
          {statItems.map((item) => (
            <div
              key={item.label}
              onClick={() => switchTab(item.tabKey)}
              style={{
                textAlign: 'center',
                padding: '10px 0',
                borderRadius: 8,
                cursor: 'pointer',
                background:
                  tab === item.tabKey
                    ? 'var(--semi-color-primary-light-default)'
                    : 'var(--semi-color-fill-0)',
              }}
            >
              <div style={{ fontSize: 20, fontWeight: 600 }}>{item.value}</div>
              <Text type="tertiary" size="small">
                {t(item.label)}
              </Text>
            </div>
          ))}
        </div>
      </Card>

      {/* ===== 核销台 ===== */}
      <Card style={{ marginBottom: 12 }} bodyStyle={{ padding: 16 }}>
        <Text strong style={{ display: 'block', marginBottom: 8 }}>
          {t('自提核销')}
        </Text>
        <Space style={{ width: '100%' }} spacing={8}>
          <Input
            size="large"
            style={{ flex: 1, minWidth: 0 }}
            placeholder={t('输入顾客出示的自提核销码')}
            value={pickupSn}
            onChange={setPickupSn}
            onEnterPress={submitWriteOff}
          />
          <Button
            theme="solid"
            type="primary"
            size="large"
            loading={writeOffLoading}
            onClick={submitWriteOff}
          >
            {t('核销')}
          </Button>
        </Space>
        <Text type="tertiary" size="small" style={{ display: 'block', marginTop: 8 }}>
          {t('扫码枪可直接扫入；核销成功后订单即完成。')}
        </Text>
      </Card>

      {/* ===== 订单处理流 ===== */}
      <Card style={{ marginBottom: 12 }} bodyStyle={{ padding: 16 }}>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 12 }}>
          {ORDER_TABS.map((item) => (
            <Button
              key={item.key || 'all'}
              theme={tab === item.key ? 'solid' : 'light'}
              type="primary"
              onClick={() => switchTab(item.key)}
            >
              {t(item.label)}
            </Button>
          ))}
        </div>
        <Spin spinning={ordersLoading && orderPage === 1}>
          {orders.length === 0 && !ordersLoading ? (
            <Empty description={t('暂无订单')} />
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {orders.map((row) => (
                <div
                  key={row.orderId}
                  style={{
                    border: '1px solid var(--semi-color-border)',
                    borderRadius: 8,
                    padding: 12,
                  }}
                >
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      gap: 8,
                    }}
                  >
                    <Text className="mono" size="small">
                      {row.orderSn}
                    </Text>
                    <Space spacing={4}>
                      {row.isPickup ? <Tag color="violet">{t('自提')}</Tag> : null}
                      {row.shippingTypeId === 3 ? <Tag color="teal">{t('同城配送')}</Tag> : null}
                      <Tag color={statusColor[row.orderStatusName] || 'grey'}>
                        {row.orderStatusName}
                      </Tag>
                    </Space>
                  </div>
                  <div style={{ margin: '8px 0' }}>
                    {(row.items || []).map((item, index) => (
                      <div
                        key={index}
                        style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '2px 0' }}
                      >
                        {item.picThumb ? (
                          <img
                            src={item.picThumb}
                            alt=""
                            style={{ width: 36, height: 36, objectFit: 'cover', borderRadius: 4 }}
                          />
                        ) : null}
                        <Text style={{ flex: 1, minWidth: 0 }} ellipsis>
                          {item.productName}
                        </Text>
                        <Text type="tertiary">× {item.quantity}</Text>
                      </div>
                    ))}
                  </div>
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      flexWrap: 'wrap',
                      gap: 6,
                    }}
                  >
                    <Text type="tertiary" size="small">
                      {row.consignee} {row.mobile} · {row.addTime}
                    </Text>
                    <Text strong style={{ color: 'var(--semi-color-danger)' }}>
                      ¥ {Number(row.totalAmount || 0).toFixed(2)}
                    </Text>
                  </div>
                  <div style={{ display: 'flex', gap: 8, marginTop: 10, flexWrap: 'wrap' }}>
                    <Button onClick={() => setDetailId(row.orderId)}>{t('详情')}</Button>
                    {row.availableActions?.deliver ? (
                      <Button theme="solid" type="primary" onClick={() => openDeliver(row)}>
                        {t('发货')}
                      </Button>
                    ) : null}
                    {row.availableActions?.confirmReceipt ? (
                      <Button theme="solid" type="tertiary" onClick={() => confirmReceipt(row)}>
                        {row.shippingTypeId === 3 ? t('完成配送') : t('确认收货')}
                      </Button>
                    ) : null}
                    {row.isPickup && row.orderStatus === 1 ? (
                      <Text type="tertiary" size="small" style={{ alignSelf: 'center' }}>
                        {t('自提单请用上方核销台核销')}
                      </Text>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Spin>
        {orders.length < orderTotal ? (
          <Button
            block
            loading={ordersLoading}
            style={{ marginTop: 12 }}
            onClick={() => setOrderPage((page) => page + 1)}
          >
            {t('加载更多')}
          </Button>
        ) : null}
      </Card>

      {/* ===== 商品速览 ===== */}
      <Card bodyStyle={{ padding: 16 }}>
        <Text strong style={{ display: 'block', marginBottom: 8 }}>
          {t('商品速览')}
        </Text>
        <Space style={{ width: '100%', marginBottom: 12 }} spacing={8}>
          <Input
            style={{ flex: 1, minWidth: 0 }}
            prefix={<IconSearch />}
            placeholder={t('搜索商品名称')}
            value={productKeyword}
            onChange={setProductKeyword}
            onEnterPress={() => {
              setProductPage(1);
              setProductQuery(productKeyword.trim());
            }}
          />
          <Button
            theme="solid"
            type="primary"
            onClick={() => {
              setProductPage(1);
              setProductQuery(productKeyword.trim());
            }}
          >
            {t('搜索')}
          </Button>
        </Space>
        <Spin spinning={productsLoading && productPage === 1}>
          {products.length === 0 && !productsLoading ? (
            <Empty description={t('暂无商品')} />
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {products.map((row) => (
                <div
                  key={row.productId}
                  style={{
                    display: 'flex',
                    gap: 10,
                    border: '1px solid var(--semi-color-border)',
                    borderRadius: 8,
                    padding: 10,
                  }}
                >
                  {row.picThumb ? (
                    <img
                      src={row.picThumb}
                      alt=""
                      style={{ width: 56, height: 56, objectFit: 'cover', borderRadius: 6 }}
                    />
                  ) : null}
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <Text ellipsis={{ showTooltip: true }} style={{ maxWidth: '100%' }}>
                      {row.productName}
                    </Text>
                    <div
                      style={{
                        display: 'flex',
                        gap: 12,
                        alignItems: 'center',
                        flexWrap: 'wrap',
                        marginTop: 4,
                      }}
                    >
                      <Text strong style={{ color: 'var(--semi-color-danger)' }}>
                        ¥ {Number(row.productPrice || 0).toFixed(2)}
                      </Text>
                      <Text
                        size="small"
                        type={row.productStock <= LOW_STOCK ? 'danger' : 'tertiary'}
                      >
                        {t('库存')} {row.productStock}
                        {row.productStock <= LOW_STOCK ? ` · ${t('库存不足')}` : ''}
                      </Text>
                      <Tag color={row.productStatus === 1 ? 'green' : 'grey'}>
                        {row.productStatus === 1 ? t('出售中') : t('已下架')}
                      </Tag>
                    </div>
                    <div
                      style={{ display: 'flex', gap: 12, alignItems: 'center', marginTop: 8, flexWrap: 'wrap' }}
                    >
                      <Space spacing={6}>
                        <Text size="small">{t('可售')}</Text>
                        <Switch
                          checked={row.saleStatus === 1}
                          loading={salePendingId === row.productId}
                          onChange={(checked) => changeSaleStatus(row, checked ? 1 : 0)}
                        />
                      </Space>
                      <Button
                        size="small"
                        loading={shelfPendingId === row.productId}
                        onClick={() => toggleShelf(row)}
                      >
                        {row.productStatus === 1 ? t('下架') : t('上架')}
                      </Button>
                      {row.skuCount <= 1 ? (
                        <Button
                          size="small"
                          onClick={() => {
                            setStockTarget(row);
                            setStockValue(row.productStock);
                          }}
                        >
                          {t('改库存')}
                        </Button>
                      ) : (
                        <Text type="tertiary" size="small">
                          {t('多规格库存请在商品管理中编辑')}
                        </Text>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Spin>
        {products.length < productTotal ? (
          <Button
            block
            loading={productsLoading}
            style={{ marginTop: 12 }}
            onClick={() => setProductPage((page) => page + 1)}
          >
            {t('加载更多')}
          </Button>
        ) : null}
      </Card>

      {/* ===== 订单详情（复用后台订单详情弹窗） ===== */}
      <OrderDetailModal
        visible={detailId !== undefined}
        orderId={detailId}
        onCancel={() => setDetailId(undefined)}
        onChanged={reloadAll}
      />

      {/* ===== 发货弹窗 ===== */}
      <Modal
        title={t('订单发货')}
        visible={deliverTarget !== null}
        onCancel={() => setDeliverTarget(null)}
        width={420}
        footer={
          <Space>
            <Button onClick={() => setDeliverTarget(null)}>{t('取消')}</Button>
            <Button theme="solid" type="primary" loading={deliverSaving} onClick={submitDeliver}>
              {t('确认发货')}
            </Button>
          </Space>
        }
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12, margin: '8px 0' }}>
          <div>
            {(deliverItems || []).map((item) => (
              <div key={item.itemId} style={{ display: 'flex', justifyContent: 'space-between' }}>
                <Text>{item.productName}</Text>
                <Text type="tertiary">
                  × {Math.max(0, Number(item.quantity) - Number(item.deliveryQuantity || 0))}
                </Text>
              </div>
            ))}
          </div>
          <Select
            style={{ width: '100%' }}
            placeholder={t('选择物流公司')}
            value={deliverForm.logisticsId}
            onChange={(value) => setDeliverForm((prev) => ({ ...prev, logisticsId: value as number }))}
            optionList={logistics.map((item) => ({ label: item.name, value: item.id }))}
          />
          <Input
            placeholder={t('物流单号')}
            value={deliverForm.trackingNo}
            onChange={(value) => setDeliverForm((prev) => ({ ...prev, trackingNo: value }))}
          />
        </div>
      </Modal>

      {/* ===== 改库存弹窗 ===== */}
      <Modal
        title={t('修改库存')}
        visible={stockTarget !== null}
        onCancel={() => setStockTarget(null)}
        width={360}
        footer={
          <Space>
            <Button onClick={() => setStockTarget(null)}>{t('取消')}</Button>
            <Button theme="solid" type="primary" loading={stockSaving} onClick={submitStock}>
              {t('保存')}
            </Button>
          </Space>
        }
      >
        <div style={{ margin: '12px 0' }}>
          <Text style={{ display: 'block', marginBottom: 8 }}>{stockTarget?.productName}</Text>
          <InputNumber
            style={{ width: '100%' }}
            min={0}
            value={stockValue}
            onChange={(value) => setStockValue(Number(value || 0))}
          />
        </div>
      </Modal>
    </div>
  );
}
