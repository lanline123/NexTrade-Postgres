// 我方新增（外卖聚合一期）：设置 → 商城设置 → 外卖聚合。
// 四段：渠道配置（凭据走配置位，密钥只进不出、空表示不修改）、门店绑定
// （ISV + 逐店授权，app_poi_code 为绑定键）、SKU 映射（渠道 SKU → 我方
// 商品，长期运维项）、外部订单（汇入落库的履约单与兜底状态，含手动
// 确认接单 / 物流同步 / 退款应答 / 重新处理）。未配置凭据时后端诚实
// 返回未启用，页面如实展示，不假成功。
import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { unixToDateTime } from '../utils/format';

const { Text, Title, Paragraph } = Typography;

interface ChannelView {
  channel: string;
  channelName: string;
  adapterReady: boolean;
  enabled: boolean;
  appId: string;
  appSecretConfigured: boolean;
  gatewayBase: string;
  configured: boolean;
  active: boolean;
}

interface Overview {
  channels: ChannelView[];
  bindingCount: number;
  pendingBindingCount: number;
  callbackBase: string;
}

interface BindingRow {
  id: number;
  channel: string;
  shopId: number;
  appPoiCode: string;
  externalPoiId: string;
  externalPoiName: string;
  authStatus: number;
  accessTokenConfigured: boolean;
  autoAccept: boolean;
  enabled: boolean;
}

interface MappingRow {
  id: number;
  channel: string;
  shopId: number;
  externalSpuId: string;
  externalSkuId: string;
  externalName: string;
  productId: number;
  skuId: number;
}

interface ExternalOrderRow {
  id: number;
  channel: string;
  channelName: string;
  externalOrderId: string;
  shopId: number;
  internalOrderId: number;
  status: string;
  totalAmount: number | string;
  consignee: string;
  mobile: string;
  lastError: string;
  addTime: number;
}

interface PageResult<T> {
  list?: T[];
  records?: T[];
  total?: number;
}

const STATUS_TAG: Record<string, { color: string; label: string }> = {
  paid: { color: 'blue', label: '待确认' },
  confirmed: { color: 'cyan', label: '已确认' },
  completed: { color: 'green', label: '已完成' },
  cancelled: { color: 'grey', label: '已取消' },
  refunding: { color: 'orange', label: '退款中' },
  refunded: { color: 'red', label: '已退款' },
  pending_binding: { color: 'amber', label: '待绑定门店' },
};

const CHANNEL_OPTIONS = [
  { value: 'meituan', label: '美团闪购' },
  { value: 'eleme', label: '淘宝闪购（二期）' },
  { value: 'douyin', label: '抖音生活服务（二期）' },
];

export default function TakeoutManage() {
  const [overview, setOverview] = useState<Overview | null>(null);
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [bindings, setBindings] = useState<BindingRow[]>([]);
  const [mappings, setMappings] = useState<MappingRow[]>([]);
  const [orders, setOrders] = useState<ExternalOrderRow[]>([]);
  const [bindingModal, setBindingModal] = useState<Partial<BindingRow> | null>(null);
  const [mappingModal, setMappingModal] = useState<Partial<MappingRow> | null>(null);
  const [detail, setDetail] = useState<(ExternalOrderRow & { rawPayload?: string }) | null>(null);
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const [ov, b, m, o] = await Promise.all([
        apiGet<Overview>('/setting/takeout/overview'),
        apiGet<PageResult<BindingRow>>('/setting/takeout/bindings'),
        apiGet<PageResult<MappingRow>>('/setting/takeout/mappings'),
        apiGet<PageResult<ExternalOrderRow>>('/setting/takeout/orders', { page: 1, pageSize: 50 }),
      ]);
      setOverview(ov);
      setBindings(b.list ?? b.records ?? []);
      setMappings(m.list ?? m.records ?? []);
      setOrders(o.list ?? o.records ?? []);
    } catch (e) {
      Toast.error(`加载失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const saveChannel = async (ch: ChannelView) => {
    try {
      const res = await apiPost<Overview>('/setting/takeout/config/save', {
        channel: ch.channel,
        enabled: ch.enabled,
        appId: ch.appId,
        appSecret: secrets[ch.channel] ?? '',
        gatewayBase: ch.gatewayBase,
      });
      setOverview(res);
      setSecrets((s) => ({ ...s, [ch.channel]: '' }));
      Toast.success('渠道配置已保存');
    } catch (e) {
      Toast.error(`保存失败：${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const patchChannel = (channel: string, patch: Partial<ChannelView>) => {
    setOverview((ov) =>
      ov
        ? { ...ov, channels: ov.channels.map((c) => (c.channel === channel ? { ...c, ...patch } : c)) }
        : ov,
    );
  };

  const saveBinding = async () => {
    if (!bindingModal) return;
    try {
      await apiPost('/setting/takeout/bindings/save', {
        id: bindingModal.id ?? 0,
        channel: bindingModal.channel ?? 'meituan',
        shopId: bindingModal.shopId ?? 0,
        appPoiCode: bindingModal.appPoiCode ?? '',
        externalPoiId: bindingModal.externalPoiId ?? '',
        externalPoiName: bindingModal.externalPoiName ?? '',
        authStatus: bindingModal.authStatus ?? 0,
        accessToken: (bindingModal as { accessToken?: string }).accessToken ?? '',
        autoAccept: bindingModal.autoAccept ?? true,
        enabled: bindingModal.enabled ?? true,
      });
      setBindingModal(null);
      Toast.success('绑定已保存');
      void reload();
    } catch (e) {
      Toast.error(`保存失败：${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const saveMapping = async () => {
    if (!mappingModal) return;
    try {
      await apiPost('/setting/takeout/mappings/save', {
        id: mappingModal.id ?? 0,
        channel: mappingModal.channel ?? 'meituan',
        shopId: mappingModal.shopId ?? 0,
        externalSpuId: mappingModal.externalSpuId ?? '',
        externalSkuId: mappingModal.externalSkuId ?? '',
        externalName: mappingModal.externalName ?? '',
        productId: mappingModal.productId ?? 0,
        skuId: mappingModal.skuId ?? 0,
      });
      setMappingModal(null);
      Toast.success('映射已保存');
      void reload();
    } catch (e) {
      Toast.error(`保存失败：${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const orderAction = async (path: string, body: Record<string, unknown>, okMsg: string) => {
    try {
      await apiPost(path, body);
      Toast.success(okMsg);
      void reload();
    } catch (e) {
      Toast.error(`操作失败：${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const openDetail = async (row: ExternalOrderRow) => {
    try {
      const d = await apiGet<ExternalOrderRow & { rawPayload?: string }>(
        '/setting/takeout/orders/detail',
        { id: row.id },
      );
      setDetail(d);
    } catch (e) {
      Toast.error(`加载详情失败：${e instanceof Error ? e.message : String(e)}`);
    }
  };

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Title heading={4} style={{ margin: 0 }}>
        外卖聚合
      </Title>
      <Paragraph type="tertiary" style={{ margin: 0 }}>
        第三方渠道订单汇入本系统统一履约（一期：美团闪购）。凭据未配置时渠道为未启用状态，回调会被诚实拒绝并留日志；
        外部订单资金在渠道侧结算，不进入平台结算与分销。配送聚合（麦芽田）等商务对接完成后另行开通。
      </Paragraph>

      <Card title="渠道配置" loading={loading}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {(overview?.channels ?? []).map((ch) => (
            <div
              key={ch.channel}
              style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}
            >
              <Text strong style={{ width: 130 }}>
                {ch.channelName}
              </Text>
              {!ch.adapterReady && <Tag color="grey">二期未接入</Tag>}
              {ch.adapterReady && ch.active && <Tag color="green">已启用</Tag>}
              {ch.adapterReady && !ch.active && <Tag color="amber">未启用</Tag>}
              <Switch
                checked={ch.enabled}
                disabled={!ch.adapterReady}
                onChange={(v) => patchChannel(ch.channel, { enabled: v })}
              />
              <Input
                placeholder="app_id"
                value={ch.appId}
                style={{ width: 180 }}
                disabled={!ch.adapterReady}
                onChange={(v) => patchChannel(ch.channel, { appId: v })}
              />
              <Input
                mode="password"
                placeholder={ch.appSecretConfigured ? 'app_secret 已配置（留空不改）' : 'app_secret 未配置'}
                value={secrets[ch.channel] ?? ''}
                style={{ width: 240 }}
                disabled={!ch.adapterReady}
                onChange={(v) => setSecrets((s) => ({ ...s, [ch.channel]: v }))}
              />
              <Input
                placeholder="网关地址（留空用默认）"
                value={ch.gatewayBase}
                style={{ width: 220 }}
                disabled={!ch.adapterReady}
                onChange={(v) => patchChannel(ch.channel, { gatewayBase: v })}
              />
              <Button disabled={!ch.adapterReady} onClick={() => void saveChannel(ch)}>
                保存
              </Button>
            </div>
          ))}
          <Text type="tertiary">
            美团回调地址（在美团开放平台应用后台按事件分别配置）：订单 {overview?.callbackBase ?? '/api/takeout/callback'}
            /meituan/order、取消 …/meituan/cancel、退款 …/meituan/refund，状态 …/meituan/status（前缀为商城站点域名）。
          </Text>
        </div>
      </Card>

      <Card
        title={`门店绑定（${bindings.length}）`}
        headerExtraContent={
          <Button
            onClick={() =>
              setBindingModal({ channel: 'meituan', authStatus: 0, autoAccept: true, enabled: true })
            }
          >
            新增绑定
          </Button>
        }
      >
        <Table
          pagination={false}
          dataSource={bindings}
          rowKey="id"
          columns={[
            { title: '渠道', dataIndex: 'channel', render: (v: string) => CHANNEL_OPTIONS.find((o) => o.value === v)?.label ?? v },
            { title: '门店ID', dataIndex: 'shopId' },
            { title: '门店编码 app_poi_code', dataIndex: 'appPoiCode' },
            { title: '渠道门店', dataIndex: 'externalPoiName' },
            {
              title: '授权',
              dataIndex: 'authStatus',
              render: (v: number) =>
                v === 1 ? <Tag color="green">已授权</Tag> : v === 2 ? <Tag color="red">已失效</Tag> : <Tag color="grey">未授权</Tag>,
            },
            { title: '令牌', dataIndex: 'accessTokenConfigured', render: (v: boolean) => (v ? '已配置' : '未配置') },
            { title: '自动接单', dataIndex: 'autoAccept', render: (v: boolean) => (v ? '开' : '关') },
            {
              title: '操作',
              render: (_: unknown, row: BindingRow) => (
                <Space>
                  <Button size="small" onClick={() => setBindingModal({ ...row })}>
                    编辑
                  </Button>
                  <Popconfirm
                    title="删除该绑定？删除后该门店的渠道订单将落入待绑定兜底。"
                    onConfirm={() =>
                      void orderAction('/setting/takeout/bindings/delete', { id: row.id }, '已删除')
                    }
                  >
                    <Button size="small" type="danger">
                      删除
                    </Button>
                  </Popconfirm>
                </Space>
              ),
            },
          ]}
        />
      </Card>

      <Card
        title={`SKU 映射（${mappings.length}）`}
        headerExtraContent={
          <Button onClick={() => setMappingModal({ channel: 'meituan' })}>新增映射</Button>
        }
      >
        <Table
          pagination={false}
          dataSource={mappings}
          rowKey="id"
          columns={[
            { title: '渠道', dataIndex: 'channel', render: (v: string) => CHANNEL_OPTIONS.find((o) => o.value === v)?.label ?? v },
            { title: '门店ID', dataIndex: 'shopId' },
            { title: '渠道SKU', dataIndex: 'externalSkuId' },
            { title: '渠道商品名', dataIndex: 'externalName' },
            { title: '我方商品ID', dataIndex: 'productId' },
            { title: '我方SKU ID', dataIndex: 'skuId' },
            {
              title: '操作',
              render: (_: unknown, row: MappingRow) => (
                <Space>
                  <Button size="small" onClick={() => setMappingModal({ ...row })}>
                    编辑
                  </Button>
                  <Popconfirm
                    title="删除该映射？"
                    onConfirm={() =>
                      void orderAction('/setting/takeout/mappings/delete', { id: row.id }, '已删除')
                    }
                  >
                    <Button size="small" type="danger">
                      删除
                    </Button>
                  </Popconfirm>
                </Space>
              ),
            },
          ]}
        />
        <Text type="tertiary">未映射的渠道商品会按渠道原始商品名落单（商品ID 为 0），不影响接单，补映射只对后续新单生效。</Text>
      </Card>

      <Card title={`外部订单（待绑定 ${overview?.pendingBindingCount ?? 0}）`}>
        <Table
          pagination={{ pageSize: 20 }}
          dataSource={orders}
          rowKey="id"
          columns={[
            { title: '渠道', dataIndex: 'channelName' },
            { title: '渠道订单号', dataIndex: 'externalOrderId' },
            { title: '门店ID', dataIndex: 'shopId' },
            { title: '内部订单ID', dataIndex: 'internalOrderId', render: (v: number) => (v > 0 ? v : '—') },
            {
              title: '状态',
              dataIndex: 'status',
              render: (v: string) => {
                const t = STATUS_TAG[v];
                return t ? <Tag color={t.color}>{t.label}</Tag> : <Tag>{v}</Tag>;
              },
            },
            { title: '金额', dataIndex: 'totalAmount' },
            { title: '收货人', dataIndex: 'consignee' },
            { title: '异常', dataIndex: 'lastError', render: (v: string) => (v ? <Text type="danger">{v}</Text> : '—') },
            { title: '时间', dataIndex: 'addTime', render: (v: number) => unixToDateTime(v) },
            {
              title: '操作',
              render: (_: unknown, row: ExternalOrderRow) => (
                <Space>
                  <Button size="small" onClick={() => void openDetail(row)}>
                    详情
                  </Button>
                  {(row.status === 'paid' || row.lastError.includes('确认')) && (
                    <Button
                      size="small"
                      onClick={() =>
                        void orderAction('/setting/takeout/orders/confirm', { id: row.id }, '已确认接单')
                      }
                    >
                      确认接单
                    </Button>
                  )}
                  {row.status === 'pending_binding' && (
                    <Button
                      size="small"
                      onClick={() =>
                        void orderAction('/setting/takeout/orders/reprocess', { id: row.id }, '已重新处理')
                      }
                    >
                      重新处理
                    </Button>
                  )}
                </Space>
              ),
            },
          ]}
        />
      </Card>

      <Modal
        title={bindingModal?.id ? '编辑绑定' : '新增绑定'}
        visible={!!bindingModal}
        onOk={() => void saveBinding()}
        onCancel={() => setBindingModal(null)}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <Select
            value={bindingModal?.channel ?? 'meituan'}
            optionList={CHANNEL_OPTIONS}
            onChange={(v) => setBindingModal((m) => ({ ...m, channel: String(v) }))}
          />
          <InputNumber
            placeholder="我方门店ID"
            value={bindingModal?.shopId}
            onChange={(v) => setBindingModal((m) => ({ ...m, shopId: Number(v ?? 0) }))}
          />
          <Input
            placeholder="门店编码 app_poi_code（与渠道后台一致）"
            value={bindingModal?.appPoiCode ?? ''}
            onChange={(v) => setBindingModal((m) => ({ ...m, appPoiCode: v }))}
          />
          <Input
            placeholder="渠道门店名"
            value={bindingModal?.externalPoiName ?? ''}
            onChange={(v) => setBindingModal((m) => ({ ...m, externalPoiName: v }))}
          />
          <Input
            placeholder="渠道门店ID（可空）"
            value={bindingModal?.externalPoiId ?? ''}
            onChange={(v) => setBindingModal((m) => ({ ...m, externalPoiId: v }))}
          />
          <Select
            value={bindingModal?.authStatus ?? 0}
            optionList={[
              { value: 0, label: '未授权' },
              { value: 1, label: '已授权' },
              { value: 2, label: '已失效' },
            ]}
            onChange={(v) => setBindingModal((m) => ({ ...m, authStatus: Number(v) }))}
          />
          <Input
            mode="password"
            placeholder={bindingModal?.accessTokenConfigured ? '授权令牌已配置（留空不改）' : '商家授权令牌 access_token'}
            onChange={(v) =>
              setBindingModal((m) => ({ ...m, accessToken: v }) as Partial<BindingRow>)
            }
          />
          <Space>
            <Text>自动接单</Text>
            <Switch
              checked={bindingModal?.autoAccept ?? true}
              onChange={(v) => setBindingModal((m) => ({ ...m, autoAccept: v }))}
            />
            <Text>启用</Text>
            <Switch
              checked={bindingModal?.enabled ?? true}
              onChange={(v) => setBindingModal((m) => ({ ...m, enabled: v }))}
            />
          </Space>
        </div>
      </Modal>

      <Modal
        title={mappingModal?.id ? '编辑映射' : '新增映射'}
        visible={!!mappingModal}
        onOk={() => void saveMapping()}
        onCancel={() => setMappingModal(null)}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <Select
            value={mappingModal?.channel ?? 'meituan'}
            optionList={CHANNEL_OPTIONS}
            onChange={(v) => setMappingModal((m) => ({ ...m, channel: String(v) }))}
          />
          <InputNumber
            placeholder="我方门店ID"
            value={mappingModal?.shopId}
            onChange={(v) => setMappingModal((m) => ({ ...m, shopId: Number(v ?? 0) }))}
          />
          <Input
            placeholder="渠道 SKU 编号（app_food_code / sku_id）"
            value={mappingModal?.externalSkuId ?? ''}
            onChange={(v) => setMappingModal((m) => ({ ...m, externalSkuId: v }))}
          />
          <Input
            placeholder="渠道商品ID（可空）"
            value={mappingModal?.externalSpuId ?? ''}
            onChange={(v) => setMappingModal((m) => ({ ...m, externalSpuId: v }))}
          />
          <Input
            placeholder="渠道商品名（辅助识别）"
            value={mappingModal?.externalName ?? ''}
            onChange={(v) => setMappingModal((m) => ({ ...m, externalName: v }))}
          />
          <InputNumber
            placeholder="我方商品ID"
            value={mappingModal?.productId}
            onChange={(v) => setMappingModal((m) => ({ ...m, productId: Number(v ?? 0) }))}
          />
          <InputNumber
            placeholder="我方 SKU ID（无规格填 0）"
            value={mappingModal?.skuId}
            onChange={(v) => setMappingModal((m) => ({ ...m, skuId: Number(v ?? 0) }))}
          />
        </div>
      </Modal>

      <Modal title="外部订单详情" visible={!!detail} footer={null} onCancel={() => setDetail(null)}>
        {detail && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <Text>渠道订单号：{detail.externalOrderId}</Text>
            <Text>收货人：{detail.consignee} {detail.mobile}</Text>
            <Text>异常：{detail.lastError || '无'}</Text>
            <Input
              readOnly
              value={detail.rawPayload ?? ''}
              style={{ fontFamily: 'monospace' }}
            />
          </div>
        )}
      </Modal>
    </div>
  );
}
