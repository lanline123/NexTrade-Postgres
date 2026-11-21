import { useEffect, useState } from 'react';
import { Button, Card, Empty, Input, Modal, Space, Table, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import FormDrawer from '../framework/FormDrawer';
import ProductPicker from '../components/ProductPicker';
import type { ColumnConfig, FormDrawerConfig } from '../framework/types';
import './auction-bids-modal.css';

const { Text } = Typography;

interface ItemRow {
  itemId: number;
  title: string;
  productName?: string;
  picThumb?: string;
  startPrice?: string | number;
  currentPrice?: string | number;
  depositAmount?: string | number;
  bidCount?: number;
  status?: number;
  statusName?: string;
  startTimeText?: string;
  endTimeText?: string;
  finalPrice?: string | number | null;
  winnerMaskedName?: string | null;
  orderId?: number | null;
  sessionId?: number | null;
  productId?: number;
  skuId?: number;
  startTime?: number;
  endTime?: number;
  buyoutPrice?: string | number | null;
  hasReserve?: boolean;
}

interface BidRow {
  bidId: number;
  username?: string;
  nickname?: string;
  bidPrice?: string | number;
  isBuyout?: number;
  statusName?: string;
  status?: number;
  ip?: string;
  addTimeText?: string;
}

interface Stats {
  totalItems: number;
  runningItems: number;
  soldItems: number;
  unsoldItems: number;
  dealRate: number;
  avgPremiumRate: number;
  totalBids: number;
  frozenDepositTotal: string | number;
  penaltyTotal: string | number;
}

const stampToText = (stamp?: number) => {
  if (!stamp) return '';
  const d = new Date(stamp * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
};

const textToStamp = (value: string) => Math.floor(new Date(value.replace(/-/g, '/')).getTime() / 1000);

const STATUS_OPTIONS = [
  { label: '全部状态', value: '' },
  { label: '草稿', value: 0 },
  { label: '预告', value: 1 },
  { label: '竞价中', value: 2 },
  { label: '已成交', value: 3 },
  { label: '流拍', value: 4 },
  { label: '已撤拍', value: 5 },
  { label: '强制结束', value: 6 },
];

const statusColor = (status?: number) => {
  switch (status) {
    case 2:
      return 'green';
    case 3:
      return 'blue';
    case 1:
      return 'orange';
    case 4:
    case 5:
    case 6:
      return 'grey';
    default:
      return 'light-blue';
  }
};

/** 拍卖商品管理（商品 → 拍卖 → 拍卖商品）。 */
export default function AuctionItems() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [stats, setStats] = useState<Stats | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<'create' | 'edit'>('create');
  const [drawerInitial, setDrawerInitial] = useState<Record<string, any>>({});
  const [editingId, setEditingId] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [sessions, setSessions] = useState<Array<{ sessionId: number; sessionName: string }>>([]);
  const [bidsItem, setBidsItem] = useState<ItemRow | null>(null);
  const [bids, setBids] = useState<BidRow[]>([]);
  const [bidsLoading, setBidsLoading] = useState(false);

  const reload = () => setReloadKey((key) => key + 1);

  useEffect(() => {
    apiGet<Stats>('/promotion/auction/stats/overview')
      .then((data) => setStats(data))
      .catch(() => setStats(null));
    apiGet<Array<{ sessionId: number; sessionName: string }>>('/promotion/auction/session/list')
      .then((data) => setSessions(Array.isArray(data) ? data : []))
      .catch(() => setSessions([]));
  }, [reloadKey]);

  const openCreate = () => {
    setDrawerMode('create');
    setEditingId(null);
    setDrawerInitial({
      productPick: [] as number[],
      title: '',
      sessionId: undefined,
      startPrice: undefined,
      reservePrice: undefined,
      buyoutPrice: undefined,
      startText: '',
      endText: '',
    });
    setDrawerOpen(true);
  };

  const openEdit = (row: ItemRow) => {
    setDrawerMode('edit');
    setEditingId(row.itemId);
    setDrawerInitial({
      productPick: row.productId ? [row.productId] : [],
      title: row.title || '',
      sessionId: row.sessionId ?? undefined,
      startPrice: row.startPrice != null ? Number(row.startPrice) : undefined,
      reservePrice: undefined,
      buyoutPrice: row.buyoutPrice != null ? Number(row.buyoutPrice) : undefined,
      startText: stampToText(row.startTime),
      endText: stampToText(row.endTime),
    });
    setDrawerOpen(true);
  };

  const save = async (payload: Record<string, any>) => {
    const productId = (payload.productPick as number[] | undefined)?.[0];
    if (drawerMode === 'create' && !productId) {
      Toast.error({ content: t('请选择拍品商品') });
      return;
    }
    if (!String(payload.title || '').trim()) {
      Toast.error({ content: t('拍品标题不能为空') });
      return;
    }
    if (!payload.startText || !payload.endText) {
      Toast.error({ content: t('请填写开始与结束时间') });
      return;
    }
    setSaving(true);
    try {
      await apiPost('/promotion/auction/item/save', {
        itemId: drawerMode === 'edit' ? editingId : undefined,
        productId: productId || undefined,
        title: payload.title,
        sessionId: payload.sessionId || undefined,
        startPrice: payload.startPrice,
        reservePrice: payload.reservePrice || undefined,
        buyoutPrice: payload.buyoutPrice || undefined,
        startTime: textToStamp(String(payload.startText)),
        endTime: textToStamp(String(payload.endText)),
      });
      Toast.success({ content: t('保存成功') });
      setDrawerOpen(false);
      reload();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const confirmAction = (title: string, fn: () => Promise<void>) => {
    Modal.confirm({
      title,
      onOk: async () => {
        try {
          await fn();
          Toast.success({ content: t('操作成功') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const loadBids = async (row: ItemRow) => {
    setBidsItem(row);
    setBidsLoading(true);
    try {
      const data = await apiGet<{ records: BidRow[] }>('/promotion/auction/bid/list', { itemId: row.itemId, pageSize: 100 });
      setBids(data?.records || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
      setBids([]);
    } finally {
      setBidsLoading(false);
    }
  };

  const voidBid = (bid: BidRow) => {
    Modal.confirm({
      title: t('确认作废这笔出价吗？作废后将按剩余出价重算当前价。'),
      onOk: async () => {
        try {
          await apiPost('/promotion/auction/bid/void', { bidId: bid.bidId });
          Toast.success({ content: t('已作废') });
          if (bidsItem) void loadBids(bidsItem);
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const columns: ColumnConfig<ItemRow>[] = [
    {
      title: t('拍品'),
      dataIndex: 'title',
      width: 280,
      render: (_: string, row: ItemRow) => (
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          {row.picThumb ? <img src={row.picThumb} alt="" style={{ width: 40, height: 40, objectFit: 'cover' }} /> : null}
          <div>
            <div>{row.title || '-'}</div>
            <Text type="tertiary" size="small">
              {row.productName || ''}
            </Text>
          </div>
        </div>
      ),
    },
    { title: t('起拍价'), dataIndex: 'startPrice', width: 100, render: (v: any) => `¥${v ?? '-'}` },
    { title: t('当前价'), dataIndex: 'currentPrice', width: 100, render: (v: any) => `¥${v ?? '-'}` },
    { title: t('保证金'), dataIndex: 'depositAmount', width: 100, render: (v: any) => `¥${v ?? '-'}` },
    { title: t('出价数'), dataIndex: 'bidCount', width: 80 },
    { title: t('开始'), dataIndex: 'startTimeText', width: 160 },
    { title: t('结束'), dataIndex: 'endTimeText', width: 160 },
    {
      title: t('状态'),
      dataIndex: 'statusName',
      width: 100,
      render: (v: string, row: ItemRow) => <Tag color={statusColor(row.status) as any}>{v ? t(v) : '-'}</Tag>,
    },
    {
      title: t('成交'),
      dataIndex: 'finalPrice',
      width: 170,
      render: (v: any, row: ItemRow) =>
        row.status === 3 ? (
          <span>
            ¥{v ?? '-'}
            {row.winnerMaskedName ? <Text type="tertiary" size="small"> · {row.winnerMaskedName}</Text> : null}
            {row.orderId ? <Text type="tertiary" size="small"> · #{row.orderId}</Text> : null}
          </span>
        ) : (
          '-'
        ),
    },
  ];

  const drawerConfig: FormDrawerConfig = {
    createApi: '/promotion/auction/item/save',
    title: t('拍卖拍品'),
    width: 640,
    labelWidth: 110,
    sections: [
      {
        key: 'basic',
        title: t('拍品设置'),
        fields: [
          {
            field: 'productPick',
            label: t('拍品商品'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <ProductPicker
                value={(values.productPick as number[]) || []}
                onChange={(ids) => formApi?.setValue('productPick', ids.slice(0, 1))}
                max={1}
              />
            ),
          },
          { field: 'title', label: t('拍品标题'), type: 'input', required: true },
          {
            field: 'sessionId',
            label: t('所属场次'),
            type: 'select',
            options: sessions.map((s) => ({ label: s.sessionName, value: s.sessionId })),
          },
          { field: 'startPrice', label: t('起拍价(元)'), type: 'number', required: true },
          {
            field: 'reservePrice',
            label: t('保留价(元)'),
            type: 'number',
            extraText: t('需在拍卖设置中开启保留价后生效；编辑时留空表示不修改原保留价'),
          },
          { field: 'buyoutPrice', label: t('一口价(元)'), type: 'number', extraText: t('留空 = 不设一口价') },
          {
            field: 'timeRange',
            label: t('竞拍时间'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <Space>
                <Input
                  style={{ width: 190 }}
                  placeholder="YYYY-MM-DD HH:mm:ss"
                  value={values.startText || ''}
                  onChange={(v) => formApi?.setValue('startText', v)}
                />
                <Input
                  style={{ width: 190 }}
                  placeholder="YYYY-MM-DD HH:mm:ss"
                  value={values.endText || ''}
                  onChange={(v) => formApi?.setValue('endText', v)}
                />
              </Space>
            ),
          },
        ],
      },
    ],
  };

  // 当前领先 = 有效出价（status=1）中的最高价者，与后端结拍口径
  // （find_top_valid_bid）一致；同价取 bidId 最大者。仅供出价记录弹窗的
  // 领先行标识与摘要条当前价呈现，纯呈现计算，不改数据、排序与接口。
  const leadingBid = bids.reduce<BidRow | null>((top, b) => {
    if (b.status !== 1) return top;
    if (!top) return b;
    const price = Number(b.bidPrice ?? 0);
    const topPrice = Number(top.bidPrice ?? 0);
    return price > topPrice || (price === topPrice && b.bidId > top.bidId) ? b : top;
  }, null);

  return (
    <div style={{ padding: 16 }}>
      {stats ? (
        <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginBottom: 12 }}>
          {[
            [t('拍品总数'), String(stats.totalItems)],
            [t('竞价中'), String(stats.runningItems)],
            [t('成交率'), `${stats.dealRate}%`],
            [t('平均溢价'), `${stats.avgPremiumRate}%`],
            [t('总出价数'), String(stats.totalBids)],
            [t('冻结保证金'), `¥${stats.frozenDepositTotal}`],
            [t('累计扣罚'), `¥${stats.penaltyTotal}`],
          ].map(([label, value]) => (
            <Card key={label} style={{ minWidth: 130 }} bodyStyle={{ padding: '10px 14px' }}>
              <Text type="tertiary" size="small">
                {label}
              </Text>
              <div style={{ fontSize: 18, fontWeight: 600 }}>{value}</div>
            </Card>
          ))}
        </div>
      ) : null}

      <ListPage<ItemRow>
        key={reloadKey}
        api="/promotion/auction/item/list"
        rowKey="itemId"
        pageSize={10}
        actionWidth={300}
        filters={[
          { field: 'keyword', label: t('拍品标题'), type: 'input', placeholder: t('输入拍品标题'), width: 220 },
          { field: 'status', label: t('状态'), type: 'select', options: STATUS_OPTIONS, width: 140 },
        ]}
        columns={columns}
        rowActions={(row) => (
          <Space spacing={2}>
            {(row.status === 0 || row.status === 1) && (
              <Button theme="borderless" size="small" onClick={() => openEdit(row)}>
                {t('编辑')}
              </Button>
            )}
            <Button theme="borderless" size="small" onClick={() => void loadBids(row)}>
              {t('出价记录')}
            </Button>
            {(row.status === 0 || row.status === 1) && (
              <Button
                theme="borderless"
                size="small"
                onClick={() =>
                  confirmAction(t('确认撤拍吗？已冻结的保证金将全额解冻。'), () =>
                    apiPost('/promotion/auction/item/cancel', { itemId: row.itemId }).then(() => undefined),
                  )
                }
              >
                {t('撤拍')}
              </Button>
            )}
            {(row.status === 1 || row.status === 2) && (
              <Button
                theme="borderless"
                size="small"
                onClick={() =>
                  confirmAction(t('确认强制结束吗？将立即解冻全部保证金且不生成订单。'), () =>
                    apiPost('/promotion/auction/item/forceEnd', { itemId: row.itemId }).then(() => undefined),
                  )
                }
              >
                {t('强制结束')}
              </Button>
            )}
            {row.status === 0 && (
              <Button
                theme="borderless"
                type="danger"
                size="small"
                onClick={() =>
                  confirmAction(t('确认删除该草稿拍品吗？'), () =>
                    apiPost('/promotion/auction/item/del', { itemId: row.itemId }).then(() => undefined),
                  )
                }
              >
                {t('删除')}
              </Button>
            )}
          </Space>
        )}
        toolbarExtra={
          <Button theme="solid" type="primary" onClick={openCreate}>
            {t('新建拍品')}
          </Button>
        }
      />

      <FormDrawer
        visible={drawerOpen}
        mode={drawerMode}
        initialValues={drawerInitial}
        config={drawerConfig}
        saving={saving}
        onCancel={() => setDrawerOpen(false)}
        onSubmit={save}
      />

      <Modal
        className="auction-bids-modal__dialog"
        centered
        title={
          <>
            {t('出价记录')}
            {bidsItem ? <span className="auction-bids-modal__title-sub">· {bidsItem.title}</span> : null}
          </>
        }
        visible={!!bidsItem}
        onCancel={() => setBidsItem(null)}
        footer={null}
        width={760}
      >
        <div className="auction-bids-modal">
          {bidsItem ? (
            <div className="auction-bids-modal__summary">
              <div className="auction-bids-modal__stat">
                <span className="auction-bids-modal__stat-label">{t('起拍价')}</span>
                <span className="auction-bids-modal__stat-value">{`¥${bidsItem.startPrice ?? '-'}`}</span>
              </div>
              <div className="auction-bids-modal__stat">
                <span className="auction-bids-modal__stat-label">{t('当前价')}</span>
                <span className="auction-bids-modal__stat-value">
                  {`¥${leadingBid ? leadingBid.bidPrice ?? '-' : bidsItem.currentPrice ?? '-'}`}
                </span>
              </div>
            </div>
          ) : null}
          <Table
            loading={bidsLoading}
            rowKey="bidId"
            pagination={false}
            dataSource={bids}
            onRow={(r: any) =>
              leadingBid && r?.bidId === leadingBid.bidId
                ? { className: 'auction-bids-modal__row--leading' }
                : {}
            }
            empty={<Empty description={t('暂无出价记录')} style={{ padding: '28px 0' }} />}
            columns={[
              {
                title: t('出价人'),
                dataIndex: 'username',
                render: (_: any, r: BidRow) => (
                  <span className="auction-bids-modal__bidder">{r.nickname || r.username || '-'}</span>
                ),
              },
              {
                title: t('出价'),
                dataIndex: 'bidPrice',
                render: (v: any, r: BidRow) => (
                  <span className="auction-bids-modal__price-cell">
                    <span className="auction-bids-modal__price">{`¥${v ?? '-'}`}</span>
                    {leadingBid && r.bidId === leadingBid.bidId ? (
                      <Tag color="green" className="auction-bids-modal__lead-tag">
                        {t('领先')}
                      </Tag>
                    ) : null}
                  </span>
                ),
              },
              {
                title: t('类型'),
                dataIndex: 'isBuyout',
                render: (v: number) => (
                  <Tag color={v === 1 ? 'orange' : 'grey'}>{v === 1 ? t('一口价') : t('竞价')}</Tag>
                ),
              },
              {
                title: t('状态'),
                dataIndex: 'statusName',
                render: (v: string, r: BidRow) => (
                  <Tag color={r.status === 2 ? 'grey' : 'green'}>{v || '-'}</Tag>
                ),
              },
              {
                title: 'IP',
                dataIndex: 'ip',
                render: (v: string) => <span className="auction-bids-modal__muted">{v}</span>,
              },
              {
                title: t('时间'),
                dataIndex: 'addTimeText',
                render: (v: string) => <span className="auction-bids-modal__muted">{v}</span>,
              },
              {
                title: t('操作'),
                dataIndex: 'actions',
                render: (_: unknown, r: BidRow) =>
                  r.status === 1 ? (
                    <Button theme="borderless" type="danger" size="small" onClick={() => voidBid(r)}>
                      {t('作废')}
                    </Button>
                  ) : null,
              },
            ]}
          />
        </div>
      </Modal>
    </div>
  );
}
