import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Input,
  Modal,
  Select,
  Space,
  Table,
  Tabs,
  TabPane,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconRefresh } from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

/**
 * 代客下单接单大厅（我方新增，二期，论坛帖 547750 余项）：
 * 一期是下单时选定履约门店；大厅模式下代客单先进待接单池，
 * 同体系营业门店在此抢单，或由下单店派单给体系内门店；接单后
 * 可标「已备货」，也可拒单退回大厅（退回后本店不再看到该单）。
 * 超时无人接单由后端定时任务按订单设置兜底（默认改派下单店）。
 * 数据作用域由后端按登录令牌的店铺作用域强制限定。
 */

type HallRow = {
  orderId: number;
  orderSn: string;
  orderStatus: number;
  payStatus: number;
  proxyShopId: number;
  proxyShopTitle?: string;
  fulfillShopId: number;
  fulfillShopTitle?: string;
  consignee: string;
  address: string;
  mobile: string;
  totalAmount: number | string;
  itemCount: number;
  firstProductName?: string;
  hallStatus: number;
  hallDeadline: number;
  acceptTime?: number;
  assignType: number;
  reassignCount: number;
  addTime: number;
};

const PAGE_SIZE = 10;

function fmtTime(epoch?: number) {
  if (!epoch) return '-';
  const d = new Date(epoch * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function deadlineText(row: HallRow, now: number) {
  if (!row.hallDeadline) return '不限时';
  const left = row.hallDeadline - now;
  if (left <= 0) return '已超时待处置';
  const m = Math.floor(left / 60);
  return m >= 60 ? `剩 ${Math.floor(m / 60)} 小时 ${m % 60} 分` : `剩 ${m} 分`;
}

export default function ProxyHall() {
  const { t } = useI18n();
  const [tab, setTab] = useState('pool');
  const [rows, setRows] = useState<HallRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  // 派单弹窗
  const [assignRow, setAssignRow] = useState<HallRow | null>(null);
  const [assignCandidates, setAssignCandidates] = useState<any[]>([]);
  const [assignTarget, setAssignTarget] = useState<number | undefined>();
  // 拒单弹窗
  const [rejectRow, setRejectRow] = useState<HallRow | null>(null);
  const [rejectReason, setRejectReason] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiGet<any>(`/proxyShopping/hall/${tab}`, {
        page,
        pageSize: PAGE_SIZE,
      });
      setRows(res?.records || res?.list || []);
      setTotal(Number(res?.total || 0));
      setNow(Math.floor(Date.now() / 1000));
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
      setRows([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [tab, page, t]);

  useEffect(() => {
    load();
  }, [load]);

  const act = async (path: string, body: Record<string, unknown>, okText: string) => {
    try {
      await apiPost(path, body);
      Toast.success({ content: t(okText) });
      load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const openAssign = async (row: HallRow) => {
    setAssignRow(row);
    setAssignTarget(undefined);
    try {
      const res = await apiGet<any>('/proxyShopping/order/check/fulfillCandidates', {});
      setAssignCandidates(Array.isArray(res) ? res : []);
    } catch {
      setAssignCandidates([]);
    }
  };

  const columns = [
    {
      title: t('订单号'),
      dataIndex: 'orderSn',
      width: 170,
      render: (v: string, row: HallRow) => (
        <div>
          <div>{v}</div>
          <Text type="tertiary" size="small">
            {fmtTime(row.addTime)}
          </Text>
        </div>
      ),
    },
    {
      title: t('下单店'),
      dataIndex: 'proxyShopTitle',
      width: 130,
      render: (v: string, row: HallRow) => v || `#${row.proxyShopId}`,
    },
    {
      title: t('商品'),
      dataIndex: 'firstProductName',
      render: (v: string, row: HallRow) => (
        <div>
          <div>{v || '-'}</div>
          <Text type="tertiary" size="small">
            {t('共 {count} 件', { count: row.itemCount })}
          </Text>
        </div>
      ),
    },
    {
      title: t('收货信息'),
      dataIndex: 'consignee',
      width: 220,
      render: (_: string, row: HallRow) => (
        <div>
          <div>
            {row.consignee} {row.mobile}
          </div>
          <Text type="tertiary" size="small">
            {row.address}
          </Text>
        </div>
      ),
    },
    {
      title: t('金额'),
      dataIndex: 'totalAmount',
      width: 100,
      render: (v: number | string) => `¥${Number(v || 0).toFixed(2)}`,
    },
    ...(tab === 'pool'
      ? [
          {
            title: t('接单截止'),
            dataIndex: 'hallDeadline',
            width: 120,
            render: (_: number, row: HallRow) => (
              <Tag color={row.hallDeadline && row.hallDeadline - now <= 0 ? 'red' : 'orange'}>
                {deadlineText(row, now)}
              </Tag>
            ),
          },
        ]
      : [
          {
            title: t('状态'),
            dataIndex: 'hallStatus',
            width: 110,
            render: (v: number, row: HallRow) => (
              <Space>
                <Tag color={v === 3 ? 'green' : 'blue'}>{v === 3 ? t('已备货') : t('已接单')}</Tag>
                {row.assignType === 3 && <Tag color="violet">{t('超时改派')}</Tag>}
              </Space>
            ),
          },
        ]),
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 210,
      render: (_: unknown, row: HallRow) =>
        tab === 'pool' ? (
          <Space>
            <Button
              theme="solid"
              size="small"
              onClick={() => act('/proxyShopping/hall/accept', { orderId: row.orderId }, '接单成功')}
            >
              {t('接单')}
            </Button>
            <Button size="small" onClick={() => openAssign(row)}>
              {t('派单')}
            </Button>
          </Space>
        ) : (
          <Space>
            {row.hallStatus === 2 && (
              <>
                <Button
                  theme="solid"
                  size="small"
                  onClick={() => act('/proxyShopping/hall/ready', { orderId: row.orderId }, '已标记备货完成')}
                >
                  {t('备货完成')}
                </Button>
                <Button
                  size="small"
                  type="danger"
                  onClick={() => {
                    setRejectRow(row);
                    setRejectReason('');
                  }}
                >
                  {t('拒单')}
                </Button>
              </>
            )}
          </Space>
        ),
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Text strong style={{ fontSize: 16 }}>
          {t('接单大厅')}
        </Text>
        <Button icon={<IconRefresh />} onClick={load}>
          {t('刷新')}
        </Button>
      </div>
      <Text type="tertiary" style={{ display: 'block', margin: '8px 0 4px' }}>
        {t('代客下单选择「接单大厅」方式的订单在此等待履约门店接单；仅同体系营业门店可见。超时无人接单将按订单设置自动处置。')}
      </Text>
      <Tabs
        activeKey={tab}
        onChange={(key) => {
          setTab(key);
          setPage(1);
        }}
      >
        <TabPane tab={t('待接单')} itemKey="pool" />
        <TabPane tab={t('我的接单')} itemKey="mine" />
      </Tabs>
      <Table
        columns={columns as any}
        dataSource={rows}
        rowKey="orderId"
        loading={loading}
        pagination={{
          currentPage: page,
          pageSize: PAGE_SIZE,
          total,
          onPageChange: (p: number) => setPage(p),
        }}
      />
      <Modal
        title={t('派单')}
        visible={!!assignRow}
        onCancel={() => setAssignRow(null)}
        onOk={() => {
          if (assignRow && assignTarget) {
            act('/proxyShopping/hall/assign', { orderId: assignRow.orderId, shopId: assignTarget }, '派单成功');
            setAssignRow(null);
          }
        }}
        okButtonProps={{ disabled: !assignTarget }}
      >
        <Text type="tertiary" style={{ display: 'block', marginBottom: 8 }}>
          {t('把订单 {orderSn} 指派给体系内门店履约', { orderSn: assignRow?.orderSn || '' })}
        </Text>
        <Select
          style={{ width: '100%' }}
          placeholder={t('选择履约门店')}
          value={assignTarget}
          optionList={assignCandidates.map((c) => ({
            label: `${c.shopTitle || ''}${c.isSelf ? `（${t('本店')}）` : ''}${
              c.distanceMeters != null ? `（${(Number(c.distanceMeters) / 1000).toFixed(1)} km）` : ''
            }`,
            value: Number(c.shopId),
          }))}
          onChange={(next: any) => setAssignTarget(Number(next) || undefined)}
        />
        {assignCandidates.length === 0 && (
          <Text type="warning" style={{ display: 'block', marginTop: 8 }}>
            {t('暂无可选门店（需以店铺账号操作且已开启跨店履约）')}
          </Text>
        )}
      </Modal>
      <Modal
        title={t('拒单退回大厅')}
        visible={!!rejectRow}
        onCancel={() => setRejectRow(null)}
        onOk={() => {
          if (rejectRow) {
            act('/proxyShopping/hall/reject', { orderId: rejectRow.orderId, reason: rejectReason }, '已退回大厅');
            setRejectRow(null);
          }
        }}
      >
        <Text type="tertiary" style={{ display: 'block', marginBottom: 8 }}>
          {t('拒单后订单退回大厅由其他门店接单，本店将不再看到该订单。')}
        </Text>
        <Input
          value={rejectReason}
          onChange={setRejectReason}
          placeholder={t('拒单原因（可选）')}
          maxLength={255}
        />
      </Modal>
    </div>
  );
}
