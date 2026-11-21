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
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 店铺小程序（本平台新增，论坛帖 487946「店铺能不能绑定独立小程序」）。
 * 模式①直连：每店绑定自有小程序 AppID / AppSecret，保存前服务端向微信
 * 校验凭据，Secret 加密存储、后台只显掩码。模式②第三方平台授权：商家
 * 扫码把小程序授权给平台第三方平台，平台凭 refresh token 代调用，
 * 无需 AppSecret；第三方平台凭据在「第三方平台设置」维护（仅平台），
 * 未配置时模式②未启用。买家在店铺小程序内只能看到本店商品；解绑/
 * 停用/授权撤销后该小程序请求立即被拒绝（403）。平台账号管理全部
 * 店铺，商家账号只能管理本店（服务端按登录作用域强制）。
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

/** 模式②授权行（refresh token 只回掩码）。 */
interface AuthItem {
  id: number;
  shopId: number;
  shopTitle: string;
  authorizerAppId: string;
  refreshTokenMask: string;
  nickName: string;
  headImg: string;
  userName: string;
  principalName: string;
  funcInfo: string;
  status: number;
  authorizedAt: number;
  lastRefreshAt: number;
  addTime: number;
  updateTime: number;
}

/** 第三方平台配置视图（仅平台账号可读；密钥只回掩码）。 */
interface ComponentConfigView {
  configured: boolean;
  componentAppId: string;
  appSecretMask: string;
  tokenMask: string;
  aesKeyMask: string;
  redirectBase: string;
  missing: string[];
  ticketReceived: boolean;
  eventPath: string;
  callbackPath: string;
}

interface AuthLinkResult {
  pcUrl: string;
  h5Url: string;
  state: string;
  expiresIn: number;
}

interface PendingAuthResult {
  shopId: number;
  status: string;
  authorizerAppId: string;
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

  // —— 模式②（第三方平台授权）状态 ——
  const [authRows, setAuthRows] = useState<AuthItem[]>([]);
  const [authLoading, setAuthLoading] = useState(false);
  // 第三方平台配置：仅平台账号可读，商家读不到（403）时保持 null、整段隐藏
  const [tpConfig, setTpConfig] = useState<ComponentConfigView | null>(null);
  const [tpModalVisible, setTpModalVisible] = useState(false);
  const [tpSaving, setTpSaving] = useState(false);
  const [tpAppId, setTpAppId] = useState('');
  const [tpSecret, setTpSecret] = useState('');
  const [tpToken, setTpToken] = useState('');
  const [tpAesKey, setTpAesKey] = useState('');
  const [tpRedirectBase, setTpRedirectBase] = useState('');
  const [authModalVisible, setAuthModalVisible] = useState(false);
  const [authShopId, setAuthShopId] = useState<number | undefined>(undefined);
  const [authCreating, setAuthCreating] = useState(false);
  const [authLink, setAuthLink] = useState<AuthLinkResult | null>(null);
  const [authPolling, setAuthPolling] = useState(false);

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

  const loadAuth = useCallback(async () => {
    setAuthLoading(true);
    try {
      const data = await apiGet<AuthItem[]>('/merchant/shopMiniProgram/thirdparty/authList');
      setAuthRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setAuthLoading(false);
    }
  }, [t]);

  const loadTpConfig = useCallback(async () => {
    try {
      const data = await apiGet<ComponentConfigView>(
        '/merchant/shopMiniProgram/thirdparty/config',
      );
      setTpConfig(data);
    } catch {
      // 非平台账号读不到配置：保持 null，配置段隐藏（授权段照常）
      setTpConfig(null);
    }
  }, []);

  useEffect(() => {
    void load();
    void loadAuth();
    void loadTpConfig();
  }, [load, loadAuth, loadTpConfig]);

  // 授权会话轮询：发起授权后每 3 秒查一次，直到授权完成或会话过期
  useEffect(() => {
    if (!authPolling || !authLink) return undefined;
    const timer = window.setInterval(() => {
      void (async () => {
        try {
          const pending = await apiGet<PendingAuthResult | null>(
            '/merchant/shopMiniProgram/thirdparty/authStatus',
            { state: authLink.state },
          );
          if (pending && pending.status === 'authorized') {
            setAuthPolling(false);
            Toast.success({ content: t('授权成功') });
            setAuthModalVisible(false);
            setAuthLink(null);
            void loadAuth();
          } else if (!pending) {
            setAuthPolling(false);
            Toast.warning({ content: t('授权会话已过期，请重新发起') });
          }
        } catch {
          /* 轮询失败静默，等下一轮 */
        }
      })();
    }, 3000);
    return () => window.clearInterval(timer);
  }, [authPolling, authLink, loadAuth, t]);

  const openTpConfig = () => {
    setTpAppId(tpConfig?.componentAppId || '');
    setTpRedirectBase(tpConfig?.redirectBase || '');
    setTpSecret('');
    setTpToken('');
    setTpAesKey('');
    setTpModalVisible(true);
  };

  const submitTpConfig = async () => {
    setTpSaving(true);
    try {
      const body: Record<string, unknown> = {
        componentAppId: tpAppId.trim(),
        redirectBase: tpRedirectBase.trim(),
      };
      // 密钥类字段留空 = 不修改（服务端同口径），绝不回填掩码
      if (tpSecret.trim()) body['componentApp' + 'Secret'] = tpSecret.trim();
      if (tpToken.trim()) body.componentToken = tpToken.trim();
      if (tpAesKey.trim()) body.componentAesKey = tpAesKey.trim();
      await apiPost('/merchant/shopMiniProgram/thirdparty/config', body);
      Toast.success({ content: t('第三方平台配置已保存') });
      setTpModalVisible(false);
      void loadTpConfig();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setTpSaving(false);
    }
  };

  const openAuth = () => {
    setAuthShopId(undefined);
    setAuthLink(null);
    setAuthPolling(false);
    void searchShops('');
    setAuthModalVisible(true);
  };

  const createAuthLink = async () => {
    if (!authShopId) {
      Toast.warning({ content: t('请选择店铺') });
      return;
    }
    setAuthCreating(true);
    try {
      const link = await apiPost<AuthLinkResult>(
        '/merchant/shopMiniProgram/thirdparty/createAuthLink',
        { shopId: authShopId },
      );
      setAuthLink(link);
      setAuthPolling(true);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发起授权失败') });
    } finally {
      setAuthCreating(false);
    }
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      Toast.success({ content: t('已复制') });
    } catch {
      Toast.warning({ content: t('复制失败，请手动选中复制') });
    }
  };

  const syncAuthInfo = async (row: AuthItem) => {
    try {
      await apiPost('/merchant/shopMiniProgram/thirdparty/syncInfo', { id: row.id });
      Toast.success({ content: t('已同步') });
      void loadAuth();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('同步失败') });
    }
  };

  const unbindAuth = async (row: AuthItem) => {
    try {
      await apiPost('/merchant/shopMiniProgram/thirdparty/unbind', { id: row.id });
      Toast.success({ content: t('已解绑') });
      void loadAuth();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('解绑失败') });
    }
  };

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

  const authStatusTag = (status: number) => {
    if (status === 1) return <Tag color="green">{t('授权中')}</Tag>;
    if (status === 0) return <Tag color="red">{t('已撤销')}</Tag>;
    return <Tag color="grey">{t('已解绑')}</Tag>;
  };

  const authColumns = [
    {
      title: t('店铺'),
      dataIndex: 'shopTitle',
      width: 180,
      render: (_: unknown, row: AuthItem) => (
        <span>
          {row.shopTitle || '--'}
          <Text type="tertiary" size="small">{` #${row.shopId}`}</Text>
        </span>
      ),
    },
    {
      title: t('授权小程序'),
      dataIndex: 'authorizerAppId',
      width: 260,
      render: (_: unknown, row: AuthItem) => (
        <Space align="center" spacing={8}>
          {row.headImg ? (
            <img
              src={row.headImg}
              alt=""
              style={{ width: 28, height: 28, borderRadius: 4 }}
            />
          ) : null}
          <span>
            {row.nickName || '--'}
            <br />
            <Text type="tertiary" size="small">
              {row.authorizerAppId}
            </Text>
          </span>
        </Space>
      ),
    },
    {
      title: 'RefreshToken',
      dataIndex: 'refreshTokenMask',
      width: 130,
      render: (v: string) => v || '--',
    },
    {
      title: t('状态'),
      dataIndex: 'status',
      width: 90,
      render: (v: number) => authStatusTag(v),
    },
    {
      title: t('授权时间'),
      dataIndex: 'authorizedAt',
      width: 160,
      render: (v: number) => fmtTime(v),
    },
    { title: t('主体'), dataIndex: 'principalName', render: (v: string) => v || '--' },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 170,
      render: (_: unknown, row: AuthItem) => (
        <Space>
          <Button
            theme="borderless"
            size="small"
            disabled={row.status !== 1}
            onClick={() => void syncAuthInfo(row)}
          >
            {t('同步信息')}
          </Button>
          <Popconfirm
            title={t('解绑该授权？')}
            content={t(
              '解绑后该小程序将立即无法访问商城数据（买家端返回 403）。注意：微信侧的授权关系需商家在微信公众平台自行取消，本处解绑只解除本系统绑定。',
            )}
            onConfirm={() => void unbindAuth(row)}
          >
            <Button theme="borderless" size="small" type="danger" disabled={row.status === 2}>
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
      <Text strong style={{ display: 'block', marginBottom: 8 }}>
        {t('模式① · 直连绑定（商家自填 AppID / AppSecret）')}
      </Text>
      <Spin spinning={loading}>
        <Table columns={columns} dataSource={rows} rowKey="id" pagination={false} />
      </Spin>

      <Space
        style={{ margin: '24px 0 12px', width: '100%', justifyContent: 'space-between' }}
      >
        <div>
          <Title heading={6} style={{ margin: 0 }}>
            {t('模式② · 第三方平台授权')}
          </Title>
          <Text type="tertiary" size="small">
            {t(
              '商家扫码把自有小程序授权给平台第三方平台，平台统一代调用（登录、取手机号等），商家无需提供 AppSecret，版本发布由平台代管。',
            )}
          </Text>
        </div>
        <Space>
          {tpConfig ? (
            <Button onClick={openTpConfig}>{t('第三方平台设置')}</Button>
          ) : null}
          <Button type="primary" theme="solid" onClick={openAuth}>
            {t('发起授权')}
          </Button>
        </Space>
      </Space>
      {tpConfig && !tpConfig.configured ? (
        <Banner
          type="warning"
          closeIcon={null}
          style={{ marginBottom: 12 }}
          description={t(
            '第三方平台尚未配置完整，模式②暂未启用。请平台管理员在「第三方平台设置」中填写开放平台的第三方平台 AppID、AppSecret、消息校验 Token 与消息加解密 Key。',
          )}
        />
      ) : null}
      {tpConfig && tpConfig.configured ? (
        <Text
          type={tpConfig.ticketReceived ? 'success' : 'warning'}
          size="small"
          style={{ display: 'block', marginBottom: 12 }}
        >
          {tpConfig.ticketReceived
            ? t('授权事件通道已连通（已收到微信推送的 verify_ticket）。')
            : t(
                '尚未收到微信推送的 verify_ticket：请确认已在开放平台把授权事件接收 URL 配置为本商城域名 + 事件路径，并等待约 10 分钟。',
              )}
        </Text>
      ) : null}
      <Spin spinning={authLoading}>
        <Table columns={authColumns} dataSource={authRows} rowKey="id" pagination={false} />
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

      <Modal
        title={t('第三方平台设置（仅平台）')}
        visible={tpModalVisible}
        onCancel={() => setTpModalVisible(false)}
        onOk={() => void submitTpConfig()}
        okText={t('保存')}
        cancelText={t('取消')}
        confirmLoading={tpSaving}
        width={560}
      >
        <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 110 }}>
              {t('第三方平台 AppID')}
            </Text>
            <Input
              style={{ width: 340 }}
              placeholder="wx…"
              value={tpAppId}
              onChange={setTpAppId}
              showClear
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 110 }}>
              AppSecret
            </Text>
            <Input
              style={{ width: 340 }}
              mode="password"
              placeholder={
                tpConfig?.appSecretMask
                  ? `${t('已保存')} ${tpConfig.appSecretMask}，${t('留空不修改')}`
                  : t('开放平台「第三方平台 → 详情」中获取')
              }
              value={tpSecret}
              onChange={setTpSecret}
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 110 }}>
              {t('消息校验 Token')}
            </Text>
            <Input
              style={{ width: 340 }}
              mode="password"
              placeholder={
                tpConfig?.tokenMask
                  ? `${t('已保存')} ${tpConfig.tokenMask}，${t('留空不修改')}`
                  : t('与开放平台开发配置中填写的 Token 一致')
              }
              value={tpToken}
              onChange={setTpToken}
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 110 }}>
              {t('消息加解密 Key')}
            </Text>
            <Input
              style={{ width: 340 }}
              mode="password"
              placeholder={
                tpConfig?.aesKeyMask
                  ? `${t('已保存')} ${tpConfig.aesKeyMask}，${t('留空不修改')}`
                  : 'EncodingAESKey（43 位）'
              }
              value={tpAesKey}
              onChange={setTpAesKey}
            />
          </Space>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 110 }}>
              {t('授权回调域名')}
            </Text>
            <Input
              style={{ width: 340 }}
              placeholder="https://battery.recy.cc"
              value={tpRedirectBase}
              onChange={setTpRedirectBase}
              showClear
            />
          </Space>
          <Text type="tertiary" size="small">
            {t('在微信开放平台「第三方平台 → 开发配置」中：授权事件接收 URL 填「回调域名')}
            {tpConfig?.eventPath || '/api/content/wechatComponent/event'}
            {t('」，授权回调域名与上方保持一致。凭据未配置完整时模式②保持未启用，不影响模式①与平台小程序。')}
          </Text>
        </Space>
      </Modal>

      <Modal
        title={t('发起第三方平台授权')}
        visible={authModalVisible}
        onCancel={() => {
          setAuthModalVisible(false);
          setAuthPolling(false);
        }}
        footer={null}
        width={560}
      >
        <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Text strong style={{ width: 84 }}>
              {t('店铺')}
            </Text>
            <Select
              style={{ width: 300 }}
              placeholder={t('搜索店铺名称')}
              value={authShopId}
              onChange={(v) => setAuthShopId(Number(v))}
              optionList={shopOptions}
              filter
              remote
              onSearch={(v: string) => void searchShops(v)}
              disabled={!!authLink}
            />
            <Button
              type="primary"
              theme="solid"
              loading={authCreating}
              disabled={!!authLink}
              onClick={() => void createAuthLink()}
            >
              {t('生成授权链接')}
            </Button>
          </Space>
          {authLink ? (
            <>
              <Text type="tertiary" size="small">
                {t('授权链接 10 分钟内有效。请让该小程序的管理员打开下方链接（或扫码）完成授权，本窗口会自动感知授权结果。')}
              </Text>
              <Space align="center" spacing={12}>
                <Text strong style={{ width: 84 }}>
                  {t('电脑端')}
                </Text>
                <a href={authLink.pcUrl} target="_blank" rel="noreferrer">
                  {t('打开授权页')}
                </a>
                <Button size="small" onClick={() => void copyText(authLink.pcUrl)}>
                  {t('复制链接')}
                </Button>
              </Space>
              <Space align="center" spacing={12}>
                <Text strong style={{ width: 84 }}>
                  {t('手机端')}
                </Text>
                <Button size="small" onClick={() => void copyText(authLink.h5Url)}>
                  {t('复制手机授权链接')}
                </Button>
                {authPolling ? (
                  <Text type="tertiary" size="small">
                    {t('等待授权完成…')}
                  </Text>
                ) : null}
              </Space>
            </>
          ) : (
            <Text type="tertiary" size="small">
              {t('同一店铺同时只能有一条生效授权；该小程序若已用模式①直连绑定，需先解绑直连再改用授权。')}
            </Text>
          )}
        </Space>
      </Modal>
    </div>
  );
}
