import { useEffect, useState } from 'react';
import {
  Avatar,
  Banner,
  Button,
  Form,
  Image,
  InputNumber,
  Modal,
  Popconfirm,
  Radio,
  RadioGroup,
  Select,
  Space,
  Switch,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconAlertTriangle, IconPlus, IconSetting } from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

const { Text } = Typography;

interface WholesaleProductRow {
  productId: number;
  productName: string;
  productSn?: string;
  picThumb?: string;
  retailPriceRange?: string;
  wholesalePriceDisplay?: string;
  moqDimension?: number;
  moqDimensionName?: string;
  moqDisplay?: string | number;
  multipleSet?: boolean | number;
  stock?: number;
  shelfStatus?: number;
}

interface ShopSwitchState {
  globalEnabled: boolean;
  shopEnabled: boolean;
  effectiveEnabled: boolean;
}

export function WholesaleProducts() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [bannerVisible, setBannerVisible] = useState(true);

  // 本店批发开关（我方新增）：仅商家账号（shop/store）显示；平台账号
  // 的批发商品归平台批发设置总开关控制，本页不出现店级开关。
  const isMerchant = (() => {
    const adminType = localStorage.getItem('adminType') || '';
    return adminType !== '' && adminType !== 'admin';
  })();
  const [shopSwitch, setShopSwitch] = useState<ShopSwitchState | null>(null);
  const [switchSaving, setSwitchSaving] = useState(false);

  useEffect(() => {
    if (!isMerchant) return;
    apiGet<ShopSwitchState>('/wholesale/setting/shopSwitch')
      .then((res) => setShopSwitch(res))
      .catch(() => setShopSwitch(null));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleShopSwitchChange = async (checked: boolean) => {
    setSwitchSaving(true);
    try {
      const res = await apiPost<ShopSwitchState>('/wholesale/setting/shopSwitch/save', {
        enabled: checked ? 1 : 0,
      });
      setShopSwitch(res);
      Toast.success({ content: t(checked ? '本店批发已开启' : '本店批发已关闭') });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSwitchSaving(false);
    }
  };

  // 添加商品弹窗
  const [addVisible, setAddVisible] = useState(false);
  const [addLoading, setAddLoading] = useState(false);
  const [addProductId, setAddProductId] = useState<number | undefined>();

  // 批发价设置弹窗
  // 后端契约（WholesalePriceSaveDTO）：保存走 priceRows[{skuId,gradeId,tierMinQty,
  // priceMode,discount,wholesalePrice}] 全量替换，没有单个 wholesalePrice 字段——
  // 旧弹窗提交 {wholesalePrice} 被 serde 忽略、priceRows 为空必报「priceRows 不能为空」，
  // 批发价永远存不了。现按 price/detail 的 SKU×等级×阶梯行打开编辑、原样回提交。
  const [priceVisible, setPriceVisible] = useState(false);
  const [priceLoading, setPriceLoading] = useState(false);
  const [currentProduct, setCurrentProduct] = useState<WholesaleProductRow | null>(null);
  const [priceDetailLoading, setPriceDetailLoading] = useState(false);
  const [priceModeOfProduct, setPriceModeOfProduct] = useState<number | undefined>(undefined);
  const [multiSpecPriceMode, setMultiSpecPriceMode] = useState<number | undefined>(undefined);
  const [gradeOptions, setGradeOptions] = useState<Array<{ gradeId: number; gradeName?: string }>>([]);
  const [priceRows, setPriceRows] = useState<
    Array<{
      key: string;
      skuId: number;
      skuLabel: string;
      gradeId: number;
      tierMinQty: number;
      priceMode: number;
      discount?: number;
      wholesalePrice?: number;
    }>
  >([]);

  // 起批量设置弹窗
  const [buyRuleVisible, setBuyRuleVisible] = useState(false);
  const [buyRuleLoading, setBuyRuleLoading] = useState(false);
  const [buyRuleForm, setBuyRuleForm] = useState({ moqDimension: 1, moq: 1 });

  // 批量设置起批量弹窗
  const [batchRuleVisible, setBatchRuleVisible] = useState(false);
  const [batchRuleLoading, setBatchRuleLoading] = useState(false);
  const [batchProductIds, setBatchProductIds] = useState<number[]>([]);
  const [batchMoq, setBatchMoq] = useState<number>(1);
  const [batchDimension, setBatchDimension] = useState<number>(1);

  // 倍数购买设置弹窗
  // 后端契约（WholesaleMultipleSaveDTO）：只收 skuMultiples[{skuId,multiple}]，
  // 旧弹窗提交单个 multiple 被忽略、skuMultiples 为空必失败。倍数现值从
  // buyRule/detail 的 skuRules 回填（旧回填读 record.multipleSet 是 bool，恒为 1）。
  const [multipleVisible, setMultipleVisible] = useState(false);
  const [multipleLoading, setMultipleLoading] = useState(false);
  const [multipleRows, setMultipleRows] = useState<Array<{ skuId: number; multiple: number }>>([]);

  const handleDelete = async (ids: number[]) => {
    try {
      await apiPost('/wholesale/product/batchDelete', { productIds: ids });
      Toast.success({ content: t('删除成功') });
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const handleAddSubmit = async () => {
    if (!addProductId) {
      Toast.warning({ content: t('请输入商品ID') });
      return;
    }
    setAddLoading(true);
    try {
      await apiPost('/wholesale/product/batchAdd', { productIds: [addProductId] });
      Toast.success({ content: t('添加成功') });
      setAddVisible(false);
      setAddProductId(undefined);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('添加失败') });
    } finally {
      setAddLoading(false);
    }
  };

  const openPriceModal = async (record: WholesaleProductRow) => {
    setCurrentProduct(record);
    setPriceRows([]);
    setPriceVisible(true);
    setPriceDetailLoading(true);
    try {
      const detail = await apiGet<{
        multiSpecPriceMode?: number;
        priceMode?: number;
        skus: Array<{
          skuId: number;
          skuSpecText?: string;
          priceRows: Array<{
            gradeId?: number;
            tierMinQty?: number;
            priceMode?: number;
            discount?: number;
            wholesalePrice?: number;
          }>;
        }>;
      }>('/wholesale/product/price/detail', { productId: record.productId });
      setMultiSpecPriceMode(detail.multiSpecPriceMode);
      setPriceModeOfProduct(detail.priceMode);
      const flat: typeof priceRows = [];
      (detail.skus || []).forEach((sku) => {
        const skuLabel = sku.skuSpecText || `SKU ${sku.skuId}`;
        if (sku.priceRows && sku.priceRows.length > 0) {
          sku.priceRows.forEach((row, idx) => {
            flat.push({
              key: `${sku.skuId}-${idx}`,
              skuId: sku.skuId,
              skuLabel,
              gradeId: row.gradeId ?? 0,
              tierMinQty: row.tierMinQty ?? 1,
              priceMode: row.priceMode ?? detail.priceMode ?? 2,
              discount: row.discount != null ? Number(row.discount) : undefined,
              wholesalePrice: row.wholesalePrice != null ? Number(row.wholesalePrice) : undefined,
            });
          });
        } else {
          flat.push({
            key: `${sku.skuId}-0`,
            skuId: sku.skuId,
            skuLabel,
            gradeId: 0,
            tierMinQty: 1,
            priceMode: detail.priceMode ?? 2,
          });
        }
      });
      setPriceRows(flat);
      // 等级选项（失败静默：仅影响新行的等级可选范围，已有行按 gradeId 回显）
      apiGet<Array<{ gradeId: number; gradeName?: string }>>('/wholesale/grade/list?withUserCount=false')
        .then((res) => setGradeOptions(Array.isArray(res) ? res : []))
        .catch(() => undefined);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('批发价加载失败') });
      setPriceVisible(false);
    } finally {
      setPriceDetailLoading(false);
    }
  };

  const handlePriceSubmit = async () => {
    if (!currentProduct) return;
    if (priceRows.length === 0) {
      Toast.warning({ content: t('请至少设置一行批发价') });
      return;
    }
    for (const row of priceRows) {
      if (!(row.tierMinQty >= 1)) {
        Toast.warning({ content: t('阶梯起量必须 ≥ 1') });
        return;
      }
      if (row.priceMode === 1 && !(Number(row.discount) >= 0.1 && Number(row.discount) <= 9.99)) {
        Toast.warning({ content: t('折扣需在 0.10 ~ 9.99 之间') });
        return;
      }
      if (row.priceMode === 2 && !(Number(row.wholesalePrice) > 0)) {
        Toast.warning({ content: t('固定批发价必须大于 0') });
        return;
      }
    }
    setPriceLoading(true);
    try {
      await apiPost('/wholesale/product/price/save', {
        productId: currentProduct.productId,
        multiSpecPriceMode,
        priceRows: priceRows.map((row) => ({
          skuId: row.skuId,
          gradeId: row.gradeId,
          tierMinQty: row.tierMinQty,
          priceMode: row.priceMode,
          discount: row.priceMode === 1 ? row.discount : undefined,
          wholesalePrice: row.priceMode === 2 ? row.wholesalePrice : undefined,
        })),
      });
      Toast.success({ content: t('批发价设置成功') });
      setPriceVisible(false);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setPriceLoading(false);
    }
  };

  const openBuyRuleModal = (record: WholesaleProductRow) => {
    setCurrentProduct(record);
    setBuyRuleForm({
      moqDimension: record.moqDimension || 1,
      moq: Number(record.moqDisplay) || 1,
    });
    setBuyRuleVisible(true);
  };

  const handleBuyRuleSubmit = async () => {
    if (!currentProduct) return;
    setBuyRuleLoading(true);
    try {
      await apiPost('/wholesale/product/buyRule/save', {
        productId: currentProduct.productId,
        // 后端 DTO 字段名是 dimension：原先发 moqDimension 被忽略、dimension=None 必报错
        dimension: buyRuleForm.moqDimension,
        moq: buyRuleForm.moq,
      });
      Toast.success({ content: t('起批量设置成功') });
      setBuyRuleVisible(false);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setBuyRuleLoading(false);
    }
  };

  const handleBatchRuleSubmit = async () => {
    if (batchProductIds.length === 0) return;
    setBatchRuleLoading(true);
    try {
      await apiPost('/wholesale/product/buyRule/batchSave', {
        productIds: batchProductIds,
        // 同单个设置：后端要 dimension，原先缺该字段必失败
        dimension: batchDimension,
        moq: batchMoq,
      });
      Toast.success({ content: t('批量设置起批量成功') });
      setBatchRuleVisible(false);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('批量设置失败') });
    } finally {
      setBatchRuleLoading(false);
    }
  };

  const openMultipleModal = async (record: WholesaleProductRow) => {
    setCurrentProduct(record);
    setMultipleRows([{ skuId: 0, multiple: 1 }]);
    setMultipleVisible(true);
    try {
      const detail = await apiGet<{ skuRules?: Array<{ skuId: number; multiple?: number }> }>(
        '/wholesale/product/buyRule/detail',
        { productId: record.productId },
      );
      const rules = (detail.skuRules || []).filter((rule) => rule.multiple != null);
      if (rules.length > 0) {
        setMultipleRows(rules.map((rule) => ({ skuId: rule.skuId, multiple: Number(rule.multiple) || 1 })));
      }
    } catch {
      // 拉不到现值时保留单行 skuId=0 兜底（后端允许 skuId=0 表示整品倍数）
    }
  };

  const handleMultipleSubmit = async () => {
    if (!currentProduct) return;
    if (multipleRows.some((row) => !(row.multiple >= 1))) {
      Toast.warning({ content: t('购买倍数必须 ≥ 1') });
      return;
    }
    setMultipleLoading(true);
    try {
      await apiPost('/wholesale/product/multiple/save', {
        productId: currentProduct.productId,
        skuMultiples: multipleRows,
      });
      Toast.success({ content: t('倍数购买设置成功') });
      setMultipleVisible(false);
      setReloadKey((k) => k + 1);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setMultipleLoading(false);
    }
  };

  const columns: ColumnConfig[] = [
    {
      title: '商品名称',
      dataIndex: 'productName',
      width: 260,
      render: (_: unknown, row: unknown) => {
        const record = row as WholesaleProductRow;
        return (
          <Space align="center" spacing={12}>
            {record.picThumb ? (
              <Image src={record.picThumb} width={40} height={40} style={{ borderRadius: 4 }} />
            ) : (
              <div
                style={{
                  width: 40,
                  height: 40,
                  borderRadius: 4,
                  backgroundColor: 'var(--semi-color-fill-1)',
                }}
              />
            )}
            <Text style={{ maxWidth: 190 }} ellipsis={{ showTooltip: true }}>
              {record.productName}
            </Text>
          </Space>
        );
      },
    },
    {
      title: '商品信息',
      dataIndex: 'productInfo',
      width: 200,
      render: (_: unknown, row: unknown) => {
        const record = row as WholesaleProductRow;
        return (
          <div style={{ display: 'flex', flexDirection: 'column', fontSize: 12 }}>
            <Text type="tertiary">{t('编码：')}{record.productSn || '--'}</Text>
            <Text type="tertiary">{t('售价：')}{record.retailPriceRange || '--'}</Text>
          </div>
        );
      },
    },
    {
      title: '批发价',
      dataIndex: 'wholesalePrice',
      width: 140,
      render: (_: unknown, row: unknown) => {
        const record = row as WholesaleProductRow;
        return record.wholesalePriceDisplay ? (
          <Text style={{ color: 'var(--semi-color-danger)' }}>{record.wholesalePriceDisplay}</Text>
        ) : (
          <Text type="tertiary">{t('未设置')}</Text>
        );
      },
    },
    {
      title: '起批维度',
      dataIndex: 'moqDimension',
      width: 100,
      render: (_: unknown, row: unknown) => {
        const record = row as WholesaleProductRow;
        return record.moqDimensionName ? (
          <Tag size="small">{record.moqDimensionName}</Tag>
        ) : (
          <Text type="tertiary">{t('未设置')}</Text>
        );
      },
    },
    {
      title: '起批数量',
      dataIndex: 'moqDisplay',
      width: 100,
      render: (v: unknown) => (v != null ? <span>{String(v)}</span> : <Text type="tertiary">{t('未设置')}</Text>),
    },
    {
      title: '倍数购买',
      dataIndex: 'multipleSet',
      width: 100,
      render: (v: unknown) => (
        v ? <Tag size="small" color="green">{t('已设置')}</Tag> : <Text type="tertiary">{t('未设置')}</Text>
      ),
    },
    {
      title: '库存',
      dataIndex: 'stock',
      width: 90,
      render: (v: unknown) => <span>{String(v ?? 0)}</span>,
    },
    {
      title: '上下架状态',
      dataIndex: 'shelfStatus',
      width: 110,
      render: (v: unknown) => (
        <Tag size="small" color={Number(v) === 1 ? 'green' : 'grey'}>
          {Number(v) === 1 ? t('上架') : t('下架')}
        </Tag>
      ),
    },
    {
      title: '操作',
      dataIndex: 'action',
      width: 280,
      render: (_: unknown, row: unknown) => {
        const record = row as WholesaleProductRow;
        return (
          <Space spacing={4}>
            <Button
              theme="borderless"
              type="primary"
              size="small"
              onClick={() => openPriceModal(record)}
            >
              {t('批发价')}
            </Button>
            <Button
              theme="borderless"
              type="primary"
              size="small"
              onClick={() => openBuyRuleModal(record)}
            >
              {t('起批量')}
            </Button>
            <Button
              theme="borderless"
              type="primary"
              size="small"
              onClick={() => openMultipleModal(record)}
            >
              {t('倍数购买')}
            </Button>
            <Popconfirm
              title={t('确认删除该批发商品？')}
              okType="danger"
              onConfirm={() => handleDelete([record.productId])}
            >
              <Button theme="borderless" type="danger" size="small">
                {t('删除')}
              </Button>
            </Popconfirm>
          </Space>
        );
      },
    },
  ];

  return (
    <div>
      {isMerchant && shopSwitch && (
        <div
          style={{
            border: '1px solid var(--semi-color-border)',
            padding: '14px 16px',
            marginBottom: 16,
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <span style={{ fontWeight: 600 }}>{t('本店批发开关')}</span>
            <Switch
              checked={shopSwitch.shopEnabled}
              loading={switchSaving}
              onChange={handleShopSwitchChange}
            />
            <span>{shopSwitch.shopEnabled ? t('已开启') : t('已关闭')}</span>
            <Text type={shopSwitch.effectiveEnabled ? 'success' : 'warning'}>
              {shopSwitch.effectiveEnabled
                ? t('当前生效中')
                : shopSwitch.globalEnabled
                  ? t('当前未生效（本店已关闭）')
                  : t('当前未生效（平台总开关已关闭）')}
            </Text>
          </div>
          {!shopSwitch.globalEnabled ? (
            <Text type="tertiary">
              {t('平台批发总开关已关闭：本店设置暂不生效，总闸打开后按本店设置生效。')}
            </Text>
          ) : !shopSwitch.shopEnabled ? (
            <Text type="tertiary">
              {t('本店批发已关闭：本店商品不展示批发规则、批发商无法按批发价购买本店商品；下方批发配置保留，重新开启即恢复。')}
            </Text>
          ) : null}
        </div>
      )}

      {bannerVisible && (
        <Banner
          type="warning"
          description={t('批发类商品与各类营销活动互斥，不可叠加优惠')}
          closeIcon
          onClose={() => setBannerVisible(false)}
          style={{ marginBottom: 16 }}
        />
      )}

      <ListPage
        key={reloadKey}
        api="/wholesale/product/page"
        rowKey="productId"
        columns={columns}
        selectable
        batchActions={(rows: WholesaleProductRow[], clear) => (
          <Space>
            <Button
              size="small"
              onClick={() => {
                setBatchProductIds(rows.map((r) => r.productId));
                setBatchRuleVisible(true);
              }}
            >
              {t('批量设置起批量')}
            </Button>
            <Popconfirm
              title={t('确认要批量删除所选批发商品吗？')}
              okType="danger"
              onConfirm={async () => {
                await handleDelete(rows.map((r) => r.productId));
                clear();
              }}
            >
              <Button type="danger" size="small">
                {t('批量删除')} ({rows.length})
              </Button>
            </Popconfirm>
          </Space>
        )}
        toolbarExtra={
          <Button icon={<IconPlus />} type="primary" onClick={() => setAddVisible(true)}>
            {t('添加批发商品')}
          </Button>
        }
        filters={[
          {
            field: 'keyword',
            label: '商品名称',
            type: 'input',
            placeholder: '输入商品名称/货号',
          },
          {
            field: 'moqDimension',
            label: '起批维度',
            type: 'select',
            placeholder: '请选择起批维度',
            options: [
              { label: 'SPU', value: 1 },
              { label: 'SKU', value: 2 },
            ],
          },
        ]}
      />

      {/* 添加商品弹窗 */}
      <Modal
        title={t('添加批发商品')}
        visible={addVisible}
        onCancel={() => setAddVisible(false)}
        onOk={handleAddSubmit}
        confirmLoading={addLoading}
        width={480}
        okText={t('确定')}
        cancelText={t('取消')}
      >
        <div>
          <div style={{ marginBottom: 6, fontWeight: 500 }}>{t('商品ID')}</div>
          <InputNumber
            style={{ width: '100%' }}
            placeholder={t('请输入商品ID')}
            value={addProductId}
            onChange={(val) => setAddProductId(Number(val) || undefined)}
          />
        </div>
      </Modal>

      {/* 批发价设置：按 SKU × 等级 × 阶梯逐行编辑（price/detail 回填、price/save 全量替换） */}
      <Modal
        title={t('批发价设置')}
        visible={priceVisible}
        onCancel={() => setPriceVisible(false)}
        onOk={handlePriceSubmit}
        confirmLoading={priceLoading}
        width={760}
        okText={t('确定')}
        cancelText={t('取消')}
      >
        <div>
          <div style={{ marginBottom: 6, fontWeight: 500 }}>{t('商品名称')}</div>
          <Text>{currentProduct?.productName || '--'}</Text>
          <div style={{ marginTop: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
            {priceDetailLoading ? (
              <Text type="tertiary">{t('加载中…')}</Text>
            ) : priceRows.length === 0 ? (
              <Text type="tertiary">{t('该商品暂无 SKU 数据')}</Text>
            ) : (
              priceRows.map((row) => (
                <div
                  key={row.key}
                  style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}
                >
                  <Text style={{ width: 130 }} ellipsis={{ showTooltip: true }}>
                    {row.skuLabel}
                  </Text>
                  <Select
                    style={{ width: 130 }}
                    value={row.gradeId}
                    optionList={[
                      { label: t('通用价'), value: 0 },
                      ...gradeOptions.map((grade) => ({
                        label: grade.gradeName || String(grade.gradeId),
                        value: grade.gradeId,
                      })),
                    ]}
                    onChange={(next) =>
                      setPriceRows((rows) =>
                        rows.map((r) => (r.key === row.key ? { ...r, gradeId: Number(next) } : r)),
                      )
                    }
                  />
                  <InputNumber
                    style={{ width: 100 }}
                    min={1}
                    placeholder={t('起量')}
                    value={row.tierMinQty}
                    onChange={(val) =>
                      setPriceRows((rows) =>
                        rows.map((r) =>
                          r.key === row.key ? { ...r, tierMinQty: Number(val) || 1 } : r,
                        ),
                      )
                    }
                  />
                  <Select
                    style={{ width: 100 }}
                    value={row.priceMode}
                    optionList={[
                      { label: t('折扣'), value: 1 },
                      { label: t('固定价'), value: 2 },
                    ]}
                    onChange={(next) =>
                      setPriceRows((rows) =>
                        rows.map((r) =>
                          r.key === row.key ? { ...r, priceMode: Number(next) } : r,
                        ),
                      )
                    }
                  />
                  {row.priceMode === 1 ? (
                    <InputNumber
                      style={{ width: 110 }}
                      min={0.1}
                      max={9.99}
                      step={0.1}
                      placeholder={t('折扣(如 8.5)')}
                      value={row.discount}
                      onChange={(val) =>
                        setPriceRows((rows) =>
                          rows.map((r) =>
                            r.key === row.key
                              ? { ...r, discount: val == null ? undefined : Number(val) }
                              : r,
                          ),
                        )
                      }
                    />
                  ) : (
                    <InputNumber
                      style={{ width: 110 }}
                      min={0}
                      step={0.01}
                      placeholder={t('批发价')}
                      value={row.wholesalePrice}
                      onChange={(val) =>
                        setPriceRows((rows) =>
                          rows.map((r) =>
                            r.key === row.key
                              ? { ...r, wholesalePrice: val == null ? undefined : Number(val) }
                              : r,
                          ),
                        )
                      }
                    />
                  )}
                  <Button
                    theme="borderless"
                    size="small"
                    onClick={() =>
                      setPriceRows((rows) => [
                        ...rows,
                        {
                          key: `${row.skuId}-${Date.now()}`,
                          skuId: row.skuId,
                          skuLabel: row.skuLabel,
                          gradeId: row.gradeId,
                          tierMinQty: row.tierMinQty + 1,
                          priceMode: row.priceMode,
                          discount: row.discount,
                          wholesalePrice: row.wholesalePrice,
                        },
                      ])
                    }
                  >
                    {t('加阶梯')}
                  </Button>
                  <Button
                    theme="borderless"
                    size="small"
                    type="danger"
                    disabled={priceRows.length <= 1}
                    onClick={() => setPriceRows((rows) => rows.filter((r) => r.key !== row.key))}
                  >
                    {t('删除')}
                  </Button>
                </div>
              ))
            )}
          </div>
          {priceModeOfProduct != null && (
            <Text type="tertiary" size="small" style={{ display: 'block', marginTop: 10 }}>
              {t('价格模式：1=按零售价打折，2=固定批发价；保存为全量替换。')}
            </Text>
          )}
        </div>
      </Modal>

      {/* 起批量设置 */}
      <Modal
        title={t('起批量设置')}
        visible={buyRuleVisible}
        onCancel={() => setBuyRuleVisible(false)}
        onOk={handleBuyRuleSubmit}
        confirmLoading={buyRuleLoading}
        width={500}
        okText={t('确定')}
        cancelText={t('取消')}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div>
            <div style={{ marginBottom: 6, fontWeight: 500 }}>{t('起批维度')}</div>
            <RadioGroup
              value={buyRuleForm.moqDimension}
              onChange={(e) => setBuyRuleForm({ ...buyRuleForm, moqDimension: Number(e.target.value) })}
            >
              <Radio value={1}>SPU</Radio>
              <Radio value={2}>SKU</Radio>
            </RadioGroup>
          </div>
          <div>
            <div style={{ marginBottom: 6, fontWeight: 500 }}>{t('起批数量')}</div>
            <InputNumber
              style={{ width: '100%' }}
              min={1}
              value={buyRuleForm.moq}
              onChange={(val) => setBuyRuleForm({ ...buyRuleForm, moq: Number(val) || 1 })}
            />
          </div>
        </div>
      </Modal>

      {/* 批量设置起批量 */}
      <Modal
        title={t('批量设置起批量')}
        visible={batchRuleVisible}
        onCancel={() => setBatchRuleVisible(false)}
        onOk={handleBatchRuleSubmit}
        confirmLoading={batchRuleLoading}
        width={480}
        okText={t('确定')}
        cancelText={t('取消')}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div>
            <div style={{ marginBottom: 6, fontWeight: 500 }}>{t('起批维度')}</div>
            <RadioGroup
              value={batchDimension}
              onChange={(e) => setBatchDimension(Number(e.target.value))}
            >
              <Radio value={1}>SPU</Radio>
              <Radio value={2}>SKU</Radio>
            </RadioGroup>
          </div>
          <div>
            <div style={{ marginBottom: 6, fontWeight: 500 }}>{t('起批数量')}</div>
            <InputNumber
              style={{ width: '100%' }}
              min={1}
              value={batchMoq}
              onChange={(val) => setBatchMoq(Number(val) || 1)}
            />
          </div>
        </div>
      </Modal>

      {/* 倍数购买设置：按 SKU 逐行（buyRule/detail 的 skuRules 回填） */}
      <Modal
        title={t('倍数购买设置')}
        visible={multipleVisible}
        onCancel={() => setMultipleVisible(false)}
        onOk={handleMultipleSubmit}
        confirmLoading={multipleLoading}
        width={480}
        okText={t('确定')}
        cancelText={t('取消')}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {multipleRows.map((row, idx) => (
            <div key={row.skuId} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Text style={{ width: 110 }}>{row.skuId === 0 ? t('整品') : `SKU ${row.skuId}`}</Text>
              <InputNumber
                style={{ flex: 1 }}
                min={1}
                value={row.multiple}
                onChange={(val) =>
                  setMultipleRows((rows) =>
                    rows.map((r, i) => (i === idx ? { ...r, multiple: Number(val) || 1 } : r)),
                  )
                }
              />
            </div>
          ))}
          <Text type="tertiary" size="small">
            {t('购买数量必须是所设倍数的整数倍。')}
          </Text>
        </div>
      </Modal>
    </div>
  );
}
