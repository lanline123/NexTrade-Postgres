import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Input,
  Space,
  Spin,
  Switch,
  Table,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 分销小店（店中店，我方新增，论坛帖 257876041323848173）。
 * 平台查看全部分销员的外壳店（店名 / 店招 / 简介）并启停其展示。
 * 禁用只关外壳展示：不动分销员身份、客户绑定与佣金（后端 salesman_shop
 * 表独立开关，与分销员禁用是两套概念）。
 */

interface ShellShopItem {
  salesmanId: number;
  userId: number;
  nickname: string;
  mobile: string;
  salesmanShopId: number;
  shopName: string;
  banner: string;
  intro: string;
  enabled: boolean;
  customized: boolean;
  updateTime: number;
}

interface PageResult<T> {
  records: T[];
  total: number;
}

function formatTs(sec: number): string {
  if (!sec) return '-';
  const d = new Date(sec * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export default function SalesmanShopManage() {
  const { t } = useI18n();
  const [rows, setRows] = useState<ShellShopItem[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [keyword, setKeyword] = useState('');
  const [loading, setLoading] = useState(false);
  const [togglingId, setTogglingId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await apiGet<PageResult<ShellShopItem>>('/salesman/shop/list', {
        page,
        size: 20,
        keyword: keyword.trim() || undefined,
      });
      setRows(Array.isArray(data?.records) ? data.records : []);
      setTotal(Number(data?.total || 0));
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
    }
  }, [page, keyword, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const toggleEnabled = async (row: ShellShopItem, next: boolean) => {
    setTogglingId(row.salesmanId);
    try {
      await apiPost('/salesman/shop/setEnabled', { salesmanId: row.salesmanId, enabled: next });
      Toast.success({ content: next ? t('已启用展示') : t('已禁用展示') });
      setRows((prev) =>
        prev.map((r) => (r.salesmanId === row.salesmanId ? { ...r, enabled: next } : r)),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    } finally {
      setTogglingId(null);
    }
  };

  const columns = [
    {
      title: t('分销员'),
      dataIndex: 'nickname',
      render: (_: unknown, row: ShellShopItem) => (
        <div>
          <div>{row.nickname || '-'}</div>
          <Text type="tertiary" size="small">
            ID {row.salesmanId}
            {row.mobile ? ` · ${row.mobile}` : ''}
          </Text>
        </div>
      ),
    },
    {
      title: t('小店店名'),
      dataIndex: 'shopName',
      render: (value: string, row: ShellShopItem) =>
        row.customized && value ? value : <Text type="tertiary">{t('未自定义（回退昵称）')}</Text>,
    },
    {
      title: t('店招图'),
      dataIndex: 'banner',
      render: (value: string) =>
        value ? (
          <img
            src={value}
            alt=""
            style={{ width: 96, height: 40, objectFit: 'cover', display: 'block' }}
          />
        ) : (
          <Text type="tertiary">-</Text>
        ),
    },
    {
      title: t('简介'),
      dataIndex: 'intro',
      render: (value: string) =>
        value ? (
          <span
            style={{
              display: 'inline-block',
              maxWidth: 260,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              verticalAlign: 'bottom',
            }}
            title={value}
          >
            {value}
          </span>
        ) : (
          <Text type="tertiary">-</Text>
        ),
    },
    {
      title: t('更新时间'),
      dataIndex: 'updateTime',
      render: (value: number) => formatTs(value),
    },
    {
      title: t('展示状态'),
      dataIndex: 'enabled',
      render: (value: boolean, row: ShellShopItem) => (
        <Space align="center">
          <Switch
            checked={value}
            loading={togglingId === row.salesmanId}
            onChange={(checked) => void toggleEnabled(row, checked)}
          />
          <Text type="tertiary" size="small">
            {value ? t('展示中') : t('已禁用')}
          </Text>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Title heading={4} style={{ marginBottom: 4 }}>
        {t('分销小店')}
      </Title>
      <Text type="tertiary">
        {t('分销员的个性化店面外壳（店中店）。禁用只关闭外壳展示，不影响分销员身份、客户绑定与佣金。')}
      </Text>
      <div style={{ margin: '16px 0', display: 'flex', gap: 8 }}>
        <Input
          placeholder={t('分销员昵称 / 手机号 / 店名')}
          value={keyword}
          onChange={(value) => setKeyword(value)}
          onEnterPress={() => setPage(1)}
          showClear
          style={{ width: 280 }}
        />
        <Button
          onClick={() => {
            setPage(1);
            void load();
          }}
        >
          {t('搜索')}
        </Button>
      </div>
      <Spin spinning={loading}>
        <Table
          columns={columns}
          dataSource={rows}
          rowKey="salesmanId"
          pagination={{
            currentPage: page,
            pageSize: 20,
            total,
            onPageChange: (p: number) => setPage(p),
          }}
        />
      </Spin>
    </div>
  );
}
