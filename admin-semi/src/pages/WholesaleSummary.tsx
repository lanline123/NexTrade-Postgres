import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Avatar,
  Button,
  Card,
  Col,
  DatePicker,
  Input,
  Row,
  Select,
  Space,
  Spin,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

// 批发汇总（我方新增，二期）：总部跨店汇总 + 门店级批发清单。
// 平台账号见全量（可按门店筛选），商家账号后端强制只见本店。
// 接口：/wholesale/summary/overview | shopPage | productPage。

interface OverviewData {
  scopeShopId: number | null;
  shopCount: number;
  configuredProductCount: number;
  wholesalerCount: number | null;
  orderCount: number;
  lineCount: number;
  totalQuantity: number;
  totalAmount: string;
  buyerCount: number;
}

interface ShopSummaryRow {
  shopId: number;
  shopTitle: string;
  wholesaleEnabled: boolean;
  configuredProductCount: number;
  orderCount: number;
  buyerCount: number;
  totalQuantity: number;
  totalAmount: string;
}

interface ShopProductRow {
  shopId: number;
  shopTitle: string;
  productId: number;
  productName?: string;
  productSn?: string;
  picThumb?: string;
  retailPriceRange?: string;
  wholesalePriceDisplay?: string;
  moqDimensionName?: string;
  moqDisplay?: number;
  multipleSet?: boolean;
  stock?: number;
  shelfStatus?: number;
  configStatus?: number;
}

interface PageResult<T> {
  total: number;
  list: T[];
}

function toSecondsRange(range: Date[]): { startTime?: number; endTime?: number } {
  if (!Array.isArray(range) || range.length < 2 || !range[0] || !range[1]) return {};
  const start = new Date(range[0]);
  start.setHours(0, 0, 0, 0);
  const end = new Date(range[1]);
  end.setHours(23, 59, 59, 0);
  return {
    startTime: Math.floor(start.getTime() / 1000),
    endTime: Math.floor(end.getTime() / 1000),
  };
}

export function WholesaleSummary() {
  const { t } = useI18n();

  // 与批发商品页同口径：adminType 非 'admin' 视为商家账号。
  const isMerchant = (() => {
    const adminType = localStorage.getItem('adminType') || '';
    return adminType !== '' && adminType !== 'admin';
  })();

  const [range, setRange] = useState<Date[]>([]);
  const [loading, setLoading] = useState(false);
  const [overview, setOverview] = useState<OverviewData | null>(null);
  const [shopRows, setShopRows] = useState<ShopSummaryRow[]>([]);

  // 门店级批发清单
  const [productShopId, setProductShopId] = useState<number | undefined>(undefined);
  const [productKeyword, setProductKeyword] = useState('');
  const [productRows, setProductRows] = useState<ShopProductRow[]>([]);
  const [productTotal, setProductTotal] = useState(0);
  const [productPage, setProductPage] = useState(1);
  const [productLoading, setProductLoading] = useState(false);
  const PRODUCT_PAGE_SIZE = 20;

  const fetchSummary = useCallback(
    async (timeRange: { startTime?: number; endTime?: number }) => {
      setLoading(true);
      try {
        const [ov, shops] = await Promise.all([
          apiGet<OverviewData>('/wholesale/summary/overview', { ...timeRange }),
          apiGet<PageResult<ShopSummaryRow>>('/wholesale/summary/shopPage', {
            ...timeRange,
            page: 1,
            pageSize: 100,
          }),
        ]);
        setOverview(ov);
        setShopRows(shops.list || []);
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
      } finally {
        setLoading(false);
      }
    },
    [t],
  );

  const fetchProducts = useCallback(
    async (page: number, shopId: number | undefined, keyword: string) => {
      setProductLoading(true);
      try {
        const res = await apiGet<PageResult<ShopProductRow>>('/wholesale/summary/productPage', {
          page,
          pageSize: PRODUCT_PAGE_SIZE,
          shopId,
          keyword: keyword || undefined,
        });
        setProductRows(res.list || []);
        setProductTotal(res.total || 0);
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
      } finally {
        setProductLoading(false);
      }
    },
    [t],
  );

  useEffect(() => {
    fetchSummary({});
    fetchProducts(1, undefined, '');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSearch = () => {
    fetchSummary(toSecondsRange(range));
  };

  const handleReset = () => {
    setRange([]);
    fetchSummary({});
  };

  const shopOptions = useMemo(
    () =>
      shopRows.map((row) => ({
        value: row.shopId,
        label: row.shopId === 0 ? t('平台') : row.shopTitle || `${t('门店')} ${row.shopId}`,
      })),
    [shopRows, t],
  );

  const cards: Array<{ label: string; value: string | number; hidden?: boolean }> = [
    { label: t('批发订单数'), value: overview?.orderCount ?? 0 },
    { label: t('批发件数'), value: overview?.totalQuantity ?? 0 },
    { label: t('批发成交金额'), value: `¥${overview?.totalAmount ?? '0'}` },
    { label: t('批发买家数'), value: overview?.buyerCount ?? 0 },
    { label: t('有批发成交的门店'), value: overview?.shopCount ?? 0, hidden: isMerchant },
    { label: t('已配置批发商品'), value: overview?.configuredProductCount ?? 0 },
    {
      label: t('启用批发商'),
      value: overview?.wholesalerCount ?? '-',
      hidden: isMerchant,
    },
  ];

  const shopColumns = [
    {
      title: t('门店'),
      dataIndex: 'shopTitle',
      render: (_: unknown, row: ShopSummaryRow) => (
        <Text strong>{row.shopId === 0 ? t('平台') : row.shopTitle || `${t('门店')} ${row.shopId}`}</Text>
      ),
    },
    {
      title: t('批发开关'),
      dataIndex: 'wholesaleEnabled',
      width: 100,
      render: (v: boolean) =>
        v ? <Tag color="green">{t('已开启')}</Tag> : <Tag color="grey">{t('已关闭')}</Tag>,
    },
    { title: t('已配置批发商品'), dataIndex: 'configuredProductCount', width: 130 },
    { title: t('批发订单数'), dataIndex: 'orderCount', width: 110 },
    { title: t('批发买家数'), dataIndex: 'buyerCount', width: 110 },
    { title: t('批发件数'), dataIndex: 'totalQuantity', width: 100 },
    {
      title: t('批发成交金额'),
      dataIndex: 'totalAmount',
      width: 140,
      render: (v: string) => <Text strong>{`¥${v}`}</Text>,
    },
  ];

  const productColumns = [
    {
      title: t('门店'),
      dataIndex: 'shopTitle',
      width: 140,
      render: (_: unknown, row: ShopProductRow) => (
        <Text>{row.shopId === 0 ? t('平台') : row.shopTitle || `${t('门店')} ${row.shopId}`}</Text>
      ),
    },
    {
      title: t('商品'),
      dataIndex: 'productName',
      render: (_: unknown, row: ShopProductRow) => (
        <Space>
          <Avatar size="small" shape="square" src={row.picThumb} />
          <div>
            <div>{row.productName}</div>
            <Text type="tertiary" size="small">
              {row.productSn}
            </Text>
          </div>
        </Space>
      ),
    },
    { title: t('零售价'), dataIndex: 'retailPriceRange', width: 150 },
    { title: t('批发价'), dataIndex: 'wholesalePriceDisplay', width: 180 },
    {
      title: t('起批量'),
      dataIndex: 'moqDisplay',
      width: 110,
      render: (_: unknown, row: ShopProductRow) =>
        `${row.moqDisplay ?? '-'}${row.moqDimensionName ? `（${row.moqDimensionName}）` : ''}`,
    },
    {
      title: t('倍数购买'),
      dataIndex: 'multipleSet',
      width: 100,
      render: (v?: boolean) => (v ? t('已设置') : t('未设置')),
    },
    { title: t('库存'), dataIndex: 'stock', width: 90 },
    {
      title: t('状态'),
      dataIndex: 'configStatus',
      width: 90,
      render: (v?: number) =>
        v === 1 ? <Tag color="green">{t('启用')}</Tag> : <Tag color="grey">{t('禁用')}</Tag>,
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Title heading={4} style={{ marginBottom: 12 }}>
        {t('批发汇总')}
      </Title>

      <Space style={{ marginBottom: 16 }}>
        <DatePicker
          type="dateRange"
          placeholder={[t('开始日期'), t('结束日期')]}
          value={range}
          onChange={(next) => setRange(Array.isArray(next) ? (next as Date[]) : [])}
          style={{ width: 280 }}
        />
        <Button theme="solid" type="primary" onClick={handleSearch}>
          {t('搜索')}
        </Button>
        <Button onClick={handleReset}>{t('重置')}</Button>
        <Text type="tertiary">{t('按订单下单时间统计，缺省为全部时间')}</Text>
      </Space>

      <Spin spinning={loading}>
        <Row gutter={12} style={{ marginBottom: 16 }}>
          {cards
            .filter((c) => !c.hidden)
            .map((c) => (
              <Col key={c.label} span={4} style={{ marginBottom: 12 }}>
                <Card>
                  <Text type="tertiary">{c.label}</Text>
                  <div style={{ fontSize: 22, fontWeight: 600, marginTop: 6 }}>{c.value}</div>
                </Card>
              </Col>
            ))}
        </Row>

        {!isMerchant && (
          <>
            <Title heading={5} style={{ margin: '8px 0' }}>
              {t('按门店汇总')}
            </Title>
            <Table
              columns={shopColumns}
              dataSource={shopRows}
              rowKey="shopId"
              pagination={false}
              empty={<Text type="tertiary">{t('暂无批发数据')}</Text>}
              style={{ marginBottom: 24 }}
            />
          </>
        )}
        {isMerchant && shopRows.length > 0 && (
          <>
            <Title heading={5} style={{ margin: '8px 0' }}>
              {t('本店批发汇总')}
            </Title>
            <Table
              columns={shopColumns.slice(1)}
              dataSource={shopRows}
              rowKey="shopId"
              pagination={false}
              style={{ marginBottom: 24 }}
            />
          </>
        )}
      </Spin>

      <Title heading={5} style={{ margin: '8px 0' }}>
        {t('门店批发商品清单')}
      </Title>
      <Space style={{ marginBottom: 12 }}>
        {!isMerchant && (
          <Select
            placeholder={t('全部门店')}
            value={productShopId}
            onChange={(v) => {
              const next = typeof v === 'number' ? v : undefined;
              setProductShopId(next);
              setProductPage(1);
              fetchProducts(1, next, productKeyword);
            }}
            optionList={shopOptions}
            showClear
            style={{ width: 200 }}
          />
        )}
        <Input
          value={productKeyword}
          onChange={(next) => setProductKeyword(next)}
          placeholder={t('商品名称/货号')}
          showClear
          style={{ width: 220 }}
        />
        <Button
          onClick={() => {
            setProductPage(1);
            fetchProducts(1, productShopId, productKeyword);
          }}
        >
          {t('查询')}
        </Button>
      </Space>
      <Table
        columns={productColumns}
        dataSource={productRows}
        rowKey="productId"
        loading={productLoading}
        pagination={{
          currentPage: productPage,
          pageSize: PRODUCT_PAGE_SIZE,
          total: productTotal,
          onPageChange: (page: number) => {
            setProductPage(page);
            fetchProducts(page, productShopId, productKeyword);
          },
        }}
        empty={<Text type="tertiary">{t('暂无批发商品')}</Text>}
      />
    </div>
  );
}
