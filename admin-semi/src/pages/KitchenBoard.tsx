import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Badge,
  Button,
  Card,
  Empty,
  Input,
  Space,
  Spin,
  Switch,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

const { Text, Title } = Typography;

interface KitchenItemVO {
  itemId: number;
  productName: string;
  picThumb: string;
  skuData: string;
  quantity: number;
}

interface KitchenOrderVO {
  orderId: number;
  orderSn: string;
  shopId: number;
  consignee: string;
  mobile: string;
  buyerNote: string;
  shippingTypeName: string;
  payTime: string;
  waitMinutes: number;
  kitchenStatus: number;
  kitchenStatusName: string;
  urgeCount: number;
  lastUrgeTime: string;
  urgeUnread: boolean;
  items: KitchenItemVO[];
}

interface KitchenBoardVO {
  pendingCount: number;
  cookingCount: number;
  doneCount: number;
  urgeUnreadCount: number;
  total: number;
  orders: KitchenOrderVO[];
}

const STATUS_META: { status: number; label: string; color: string }[] = [
  { status: 0, label: '待制作', color: 'orange' },
  { status: 1, label: '制作中', color: 'blue' },
  { status: 2, label: '已出单', color: 'green' },
];

/** 等待时长颜色：<10 分钟常规、10–20 橙、>20 红。 */
function waitColor(min: number): string {
  if (min >= 20) return '#d92d20';
  if (min >= 10) return '#dc6803';
  return '#667085';
}

/**
 * 后厨出单（订单 → 订单管理 → 后厨出单，我方新增，论坛帖 817927）。
 *
 * 已付款待发货订单按后厨状态三列看板：待制作 → 制作中 → 已出单
 * （出单只代表后厨完工，发货/核销仍走既有链）。买家催菜的订单
 * 带红色催菜标记并置顶，未读催菜需点「已知晓」清除；看板每
 * 30 秒自动刷新（可关）。店铺账号只看本店，平台可看全部。
 */
export default function KitchenBoard() {
  const [board, setBoard] = useState<KitchenBoardVO | null>(null);
  const [loading, setLoading] = useState(true);
  const [keyword, setKeyword] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [actingId, setActingId] = useState<number>(0);
  const keywordRef = useRef('');
  keywordRef.current = keyword;

  const load = useCallback(async (silent = false) => {
    if (!silent) setLoading(true);
    try {
      const data = await apiGet<KitchenBoardVO>('/order/kitchen/board', {
        page: 1,
        size: 200,
        keyword: keywordRef.current || undefined,
      });
      setBoard(data);
    } catch {
      if (!silent) Toast.error('加载后厨看板失败');
    } finally {
      if (!silent) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!autoRefresh) return;
    const timer = setInterval(() => void load(true), 30_000);
    return () => clearInterval(timer);
  }, [autoRefresh, load]);

  const grouped = useMemo(() => {
    const map = new Map<number, KitchenOrderVO[]>();
    for (const meta of STATUS_META) map.set(meta.status, []);
    for (const o of board?.orders ?? []) {
      map.get(o.kitchenStatus)?.push(o);
    }
    return map;
  }, [board]);

  const counts = useMemo(
    () => ({
      0: board?.pendingCount ?? 0,
      1: board?.cookingCount ?? 0,
      2: board?.doneCount ?? 0,
    }),
    [board],
  );

  const transition = async (order: KitchenOrderVO, target: number) => {
    setActingId(order.orderId);
    try {
      await apiPost('/order/kitchen/transition', {
        orderId: order.orderId,
        targetStatus: target,
      });
      Toast.success(target === 1 ? '已开始制作' : '已出单');
      await load(true);
    } catch {
      Toast.error('操作失败，请刷新后重试');
    } finally {
      setActingId(0);
    }
  };

  const ackUrge = async (order: KitchenOrderVO) => {
    setActingId(order.orderId);
    try {
      await apiPost('/order/kitchen/ackUrge', { orderId: order.orderId });
      await load(true);
    } catch {
      Toast.error('操作失败，请刷新后重试');
    } finally {
      setActingId(0);
    }
  };

  const renderCard = (order: KitchenOrderVO) => (
    <Card
      key={order.orderId}
      style={{
        marginBottom: 12,
        borderRadius: 0,
        border: order.urgeUnread ? '1px solid #d92d20' : undefined,
        background: order.urgeUnread ? '#fff7f5' : undefined,
      }}
      bodyStyle={{ padding: 12 }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Text strong>{order.orderSn}</Text>
        <Text style={{ color: waitColor(order.waitMinutes), fontWeight: 600 }}>
          等待 {order.waitMinutes} 分钟
        </Text>
      </div>
      <div style={{ marginTop: 4, display: 'flex', gap: 6, flexWrap: 'wrap', alignItems: 'center' }}>
        {order.shopId > 0 ? <Tag color="grey">店铺 {order.shopId}</Tag> : null}
        {order.shippingTypeName ? <Tag color="cyan">{order.shippingTypeName}</Tag> : null}
        {order.urgeCount > 0 ? (
          <Tag color="red">
            催菜 ×{order.urgeCount}
            {order.lastUrgeTime ? ` · 最近 ${order.lastUrgeTime.slice(11)}` : ''}
          </Tag>
        ) : null}
      </div>
      <div style={{ marginTop: 8 }}>
        {order.items.map((it) => (
          <div
            key={it.itemId}
            style={{ display: 'flex', justifyContent: 'space-between', padding: '2px 0' }}
          >
            <Text>
              {it.productName}
              {it.skuData ? <Text type="tertiary">（{it.skuData}）</Text> : null}
            </Text>
            <Text strong>×{it.quantity}</Text>
          </div>
        ))}
      </div>
      {order.buyerNote ? (
        <div
          style={{
            marginTop: 8,
            background: '#fffaeb',
            border: '1px solid #fedf89',
            padding: '6px 8px',
            fontSize: 13,
          }}
        >
          备注：{order.buyerNote}
        </div>
      ) : null}
      <div style={{ marginTop: 8 }}>
        <Text type="tertiary" size="small">
          {order.consignee} {order.mobile} · 付款 {order.payTime}
        </Text>
      </div>
      <div style={{ marginTop: 10, display: 'flex', gap: 8 }}>
        {order.kitchenStatus === 0 ? (
          <Button
            theme="solid"
            loading={actingId === order.orderId}
            onClick={() => void transition(order, 1)}
          >
            开始制作
          </Button>
        ) : null}
        {order.kitchenStatus === 1 ? (
          <Button
            theme="solid"
            loading={actingId === order.orderId}
            onClick={() => void transition(order, 2)}
          >
            出单完成
          </Button>
        ) : null}
        {order.kitchenStatus === 0 ? (
          <Button loading={actingId === order.orderId} onClick={() => void transition(order, 2)}>
            直接出单
          </Button>
        ) : null}
        {order.urgeUnread ? (
          <Button type="danger" loading={actingId === order.orderId} onClick={() => void ackUrge(order)}>
            已知晓催菜
          </Button>
        ) : null}
        {order.kitchenStatus === 2 ? <Tag color="green">已出单 · 待发货/核销</Tag> : null}
      </div>
    </Card>
  );

  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <Title heading={4} style={{ marginBottom: 4 }}>
            后厨出单
            {(board?.urgeUnreadCount ?? 0) > 0 ? (
              <Badge count={board?.urgeUnreadCount} style={{ marginLeft: 10 }} />
            ) : null}
          </Title>
          <Text type="tertiary">
            已付款待发货订单的后厨队列：待制作 → 制作中 → 已出单。出单后订单回归发货/核销流程；带红色标记的是被买家催菜的订单。
          </Text>
        </div>
        <Space align="center">
          <Text type="tertiary">自动刷新</Text>
          <Switch checked={autoRefresh} onChange={setAutoRefresh} />
          <Input
            placeholder="按订单号搜索"
            value={keyword}
            onChange={setKeyword}
            onEnterPress={() => void load()}
            style={{ width: 200, borderRadius: 0 }}
            showClear
          />
          <Button onClick={() => void load()}>刷新</Button>
        </Space>
      </div>
      <Spin spinning={loading}>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
            gap: 16,
            marginTop: 16,
            alignItems: 'start',
          }}
        >
          {STATUS_META.map((meta) => (
            <div key={meta.status}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
                <Tag color={meta.color as never} size="large">
                  {meta.label}
                </Tag>
                <Text strong>{counts[meta.status as 0 | 1 | 2]}</Text>
              </div>
              {(grouped.get(meta.status) ?? []).length === 0 ? (
                <Empty description="暂无订单" style={{ padding: 24 }} />
              ) : (
                (grouped.get(meta.status) ?? []).map(renderCard)
              )}
            </div>
          ))}
        </div>
      </Spin>
    </div>
  );
}
