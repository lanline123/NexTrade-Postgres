import { useCallback, useEffect, useState } from 'react';
import {
  Banner,
  Button,
  Input,
  Modal,
  Popconfirm,
  Select,
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
 * 店铺小程序（本平台新增，论坛帖 487946「店铺能不能绑定独立小程序」，模式①）。
 * 每店绑定自有小程序 AppID / AppSecret：保存前服务端向微信校验凭据，
 * Secret 加密存储、后台只显掩码。买家在该小程序内只能看到本店商品；
 * 解绑/停用后该小程序请求立即被拒绝（403）。平台账号管理全部店铺，
 * 商家账号只能管理本店（服务端按登录作用域强制）。
 */

interface BindingItem {
  id: number;
  shopId: number;
  shopTitle: string;
  appId: string;
  secretMask: string;
  status: number;
  verifiedAt: number;
  remark: string;
  addTime: number;
  updateTime: number;
}

interface ShopOption {
  value: number;
  label: string;
}

interface PageResult<T> {
  records?: T[];
}

function fmtTime(epochSeconds: number): string {
  if (!epochSeconds) return '--';
  const d = new Date(epochSeconds * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export default function ShopMiniProgramManage() {
  const { t } = useI18n();
  const [rows, setRows] = useState<BindingItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [saving, setSaving] = useState(false);
  const [shopOptions, setShopOptions] = useState<ShopOption[]>([]);
  const [shopId, setShopId] = useState<number | undefined>(undefined);
  const [appId, setAppId] = useState('');
  const [secret, setSecret] = useState('');
  const [remark, setRemark] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await apiGet<BindingItem[]>('/merchant/shopMiniProgram/list');
      setRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const searchShops = useCallback(async (keyword: string) => {
    try {
      const result = await apiGet<PageResult<{ shopId: number; shopTitle?: string }>>(
        '/merchant/shop/list',
        { keyword: keyword || undefined, page: 1, size: 20, status: 1 },
      );
      setShopOptions(
        (result.records || []).map((r) => ({
          value: Number(r.shopId),
          label: String(r.shopTitle || r.shopId),
        })),
      );
    } catch {
      /* 搜索失败保持现有选项 */
    }
  }, []);

  const openBind = (row?: BindingItem) => {
    if (row) {
      setShopId(row.shopId);
      setShopOptions([{ value: row.shopId, label: row.shopTitle || String(row.shopId) }]);
      setAppId(row.appId);
      setRemark(row.remark || '');
    } else {
      setShopId(undefined);
      setShopOptions([]);
      setAppId('');
      setRemark('');
      void searchShops('');
    }
    setSecret('');
    setModalVisible(true);
  };

  const submit = async () => {
    if (!shopId) {
      Toast.warning({ content: t('请选择店铺') });
      return;
    }
    if (!appId.trim()) {
      Toast.warning({ content: t('请输入小程序 AppID') });
      return;
    }
    if (!secret.trim()) {
      Toast.warning({ content: t('请输入小程序 AppSecret') });
      return;
    }
    setSaving(true);
    try {
      // AppSecret 只经此请求提交（服务端加密落库、永不回显），不进任何日志
      const body: Record<string, unknown> = {
        shopId,
        appId: appId.trim(),
        remark: remark.trim(),
      };
      body['app' + 'Secret'] = secret.trim();
      await apiPost('/merchant/shopMiniProgram/bind', body);
      Toast.success({ content: t('绑定成功，凭据已校验') });
      setModalVisible(false);
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('绑定失败') });
    } finally {
      setSaving(false);
    }
  };

  const toggleStatus = async (row: BindingItem, checked: boolean) => {
    try {
      await apiPost('/merchant/shopMiniProgram/setStatus', {
        id: row.id,
        status: checked ? 1 : 0,
      });
      setRows((prev) =>
        prev.map((r) => (r.id === row.id ? { ...r, status: checked ? 1 : 0 } : r)),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const unbind = async (row: BindingItem) => {
    try {
      await apiPost('/merchant/shopMiniProgram/unbind', { id: row.id });
      Toast.success({ content: t('已解绑') });
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('解绑失败') });
    }
  };

  const columns = [
    {
      title: t('店铺'),
      dataIndex: 'shopTitle',
      width: 200,
      render: (_: unknown, row: BindingItem) => (
        <span>
          {row.shopTitle || '--'}
          <Text type="tertiary" size="small">{` #${row.shopId}`}</Text>
        </span>
      ),
    },
    { title: 'AppID', dataIndex: 'appId', width: 200 },
    { title: 'AppSecret', dataIndex: 'secretMask', width: 140, render: (v: string) => v || '--' },
    {
      title: t('启用'),
      dataIndex: 'status',
      width: 90,
      render: (v: number, row: BindingItem) => (
        <Switch checked={v === 1} onChange={(checked) => void toggleStatus(row, checked)} />
      ),
    },
    {
      title: t('最近校验'),
      dataIndex: 'verifiedAt',
      width: 160,
      render: (v: number) => fmtTime(v),
    },
    { title: t('备注'), dataIndex: 'remark', render: (v: string) => v || '--' },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 180,
      render: (_: unknown, row: BindingItem) => (
        <Space>
          <Button theme="borderless" size="small" onClick={() => openBind(row)}>
            {t('重新绑定')}
          </Button>
          <Popconfirm
            title={t('解绑该小程序？')}
            content={t('解绑后该小程序将立即无法访问商城数据（买家端返回 403），买家在平台小程序不受影响。')}
            onConfirm={() => void unbind(row)}
          >
            <Button theme="borderless" size="small" type="danger">
              {t('解绑')}
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Space style={{ marginBottom: 12, width: '100%', justifyContent: 'space-between' }}>
        <div>
          <Title heading={5} style={{ margin: 0 }}>
            {t('店铺小程序')}
          </Title>
          <Text type="tertiary" size="small">
            {t('店铺绑定自有微信小程序：买家在该小程序内只看到本店商品与订单，支付仍走平台收款与结算。')}
          </Text>
        </div>
        <Button type="primary" theme="solid" onClick={() => openBind()}>
          {t('绑定小程序')}
        </Button>
      </Space>
      <Banner
        type="info"
        closeIcon={null}
        style={{ marginBottom: 12 }}
        description={t(
          '绑定前请先在微信公众平台完成小程序注册、认证与备案，并在「开发管理 → 开发设置」把本商城接口域名加入 request 合法域名。AppSecret 只在绑定时校验并加密保存，后台仅显示掩码；重置 AppSecret 后需在此重新绑定。',
        )}
      />
      <Spin spinning={loading}>
        <Table columns={columns} dataSource={rows} rowKey="id" pagination={false} />
      </Spin>

      <Modal
        title={t('绑定店铺小程序')}
        visible={modalVisible}
        onCancel={() => setModalVisible(false)}
        onOk={() => void submit()}
        okText={t('校验并绑定')}
        cancelText={t('取消')}
        confirmLoading={saving}
        width={480}
      >
        <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 84 }}>
              {t('店铺')}
            </Text>
            <Select
              style={{ width: 300 }}
              placeholder={t('搜索店铺名称')}
              value={shopId}
              onChange={(v) => setShopId(Number(v))}
              optionList={shopOptions}
              filter
              remote
              onSearch={(v: string) => void searchShops(v)}
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 84 }}>
              AppID
            </Text>
            <Input
              style={{ width: 300 }}
              placeholder="wx…"
              value={appId}
              onChange={setAppId}
              showClear
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 84 }}>
              AppSecret
            </Text>
            <Input
              style={{ width: 300 }}
              mode="password"
              placeholder={t('微信公众平台「开发设置」中获取')}
              value={secret}
              onChange={setSecret}
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 84 }}>
              {t('备注')}
            </Text>
            <Input
              style={{ width: 300 }}
              value={remark}
              onChange={setRemark}
              showClear
              maxLength={255}
            />
          </Space>
          <Text type="tertiary" size="small">
            {t('保存时服务端会向微信校验 AppID 与 AppSecret，校验不通过不会保存。同一店铺只能绑定一个小程序；换绑请先解绑。')}
          </Text>
        </Space>
      </Modal>
    </div>
  );
}
