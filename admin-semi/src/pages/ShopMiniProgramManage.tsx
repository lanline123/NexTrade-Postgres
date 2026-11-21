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
 * 未配置时模式②未启用。三期代发布：平台模板库 + 为已授权店铺代提交
 * 代码/提审/发布/回退，商家可对本店自助发起同一套动作。买家在店铺小程序内只能看到本店商品；解绑/
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

/** 模式②代发布版本状态（三期；auditStatus 为微信口径 -1/0/1/2/3/4）。 */
interface VersionStateItem {
  id: number;
  authorizerId: number;
  shopId: number;
  shopTitle: string;
  authorizerAppId: string;
  nickName: string;
  headImg: string;
  authStatus: number;
  templateId: number;
  userVersion: string;
  userDesc: string;
  extJson: string;
  auditId: number;
  auditStatus: number;
  auditReason: string;
  auditScreenshot: string;
  auditUserVersion: string;
  submitAuditTime: number;
  onlineUserVersion: string;
  stage: number;
  updateTime: number;
}

interface ReleaseLogItem {
  id: number;
  action: string;
  detail: string;
  success: number;
  errMsg: string;
  operatorType: string;
  operatorShopId: number;
  addTime: number;
}

interface TemplateDraftItem {
  draftId: number;
  userVersion: string;
  userDesc: string;
  sourceMiniprogramAppid: string;
  sourceMiniprogram: string;
  createTime: number;
}

interface CodeTemplateItem {
  templateId: number;
  userVersion: string;
  userDesc: string;
  sourceMiniprogramAppid: string;
  sourceMiniprogram: string;
  createTime: number;
}

interface AuditCategoryItem {
  firstClass: string;
  secondClass: string;
  firstId: number;
  secondId: number;
}

interface HistoryVersionItem {
  appVersion: number;
  userVersion: string;
  userDesc: string;
  commitTime: number;
}

/** 提审项编辑行（类目以 firstId_secondId 为键回查类目表）。 */
interface AuditItemDraft {
  address: string;
  categoryKey: string;
  title: string;
  tag: string;
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

  // —— 模式② 代提审/代发布（三期）状态 ——
  const [releaseRows, setReleaseRows] = useState<VersionStateItem[]>([]);
  const [releaseLoading, setReleaseLoading] = useState(false);
  // 模板库仅平台可读：读到才显示整段（商家 403 时保持隐藏）
  const [tplVisible, setTplVisible] = useState(false);
  const [drafts, setDrafts] = useState<TemplateDraftItem[]>([]);
  const [templates, setTemplates] = useState<CodeTemplateItem[]>([]);
  const [relModalVisible, setRelModalVisible] = useState(false);
  const [relTarget, setRelTarget] = useState<{ id: number; label: string } | null>(null);
  const [relState, setRelState] = useState<VersionStateItem | null>(null);
  const [relLogs, setRelLogs] = useState<ReleaseLogItem[]>([]);
  const [relPages, setRelPages] = useState<string[]>([]);
  const [relCategories, setRelCategories] = useState<AuditCategoryItem[]>([]);
  const [relHistory, setRelHistory] = useState<HistoryVersionItem[]>([]);
  const [relBusy, setRelBusy] = useState(false);
  const [commitTemplateId, setCommitTemplateId] = useState<number | undefined>(undefined);
  const [commitVersion, setCommitVersion] = useState('');
  const [commitDesc, setCommitDesc] = useState('');
  const [auditItems, setAuditItems] = useState<AuditItemDraft[]>([
    { address: '', categoryKey: '', title: '', tag: '' },
  ]);
  const [revertAppVersion, setRevertAppVersion] = useState<number>(-1);

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

  const loadRelease = useCallback(async () => {
    setReleaseLoading(true);
    try {
      const data = await apiGet<VersionStateItem[]>(
        '/merchant/shopMiniProgram/release/overview',
      );
      setReleaseRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setReleaseLoading(false);
    }
  }, [t]);

  const loadTemplates = useCallback(async () => {
    // 模板列表平台/商家可读（发布弹窗选模板用）；草稿箱仅平台可读，
    // 读到才显示模板库管理段（商家 403 时该段隐藏，列表照常加载）
    try {
      const tpl = await apiGet<CodeTemplateItem[]>(
        '/merchant/shopMiniProgram/release/templates/list',
      );
      setTemplates(Array.isArray(tpl) ? tpl : []);
    } catch {
      setTemplates([]);
    }
    try {
      const d = await apiGet<TemplateDraftItem[]>(
        '/merchant/shopMiniProgram/release/templates/drafts',
      );
      setDrafts(Array.isArray(d) ? d : []);
      setTplVisible(true);
    } catch {
      setTplVisible(false);
    }
  }, []);

  useEffect(() => {
    void load();
    void loadAuth();
    void loadTpConfig();
    void loadRelease();
    void loadTemplates();
  }, [load, loadAuth, loadTpConfig, loadRelease, loadTemplates]);

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

  // —— 代提审/代发布（三期）动作 ——

  const refreshRelease = useCallback(
    async (authorizerId: number) => {
      const [st, logs] = await Promise.all([
        apiGet<VersionStateItem>('/merchant/shopMiniProgram/release/state', {
          authorizerId,
        }).catch(() => null),
        apiGet<ReleaseLogItem[]>('/merchant/shopMiniProgram/release/logs', {
          authorizerId,
        }).catch(() => [] as ReleaseLogItem[]),
      ]);
      if (st) setRelState(st);
      setRelLogs(Array.isArray(logs) ? logs : []);
      // 取数接口在未上传代码等场景会回微信错误（如 86001），各自静默置空
      try {
        const p = await apiGet<string[]>('/merchant/shopMiniProgram/release/pages', {
          authorizerId,
        });
        setRelPages(Array.isArray(p) ? p : []);
      } catch {
        setRelPages([]);
      }
      try {
        const c = await apiGet<AuditCategoryItem[]>(
          '/merchant/shopMiniProgram/release/categories',
          { authorizerId },
        );
        setRelCategories(Array.isArray(c) ? c : []);
      } catch {
        setRelCategories([]);
      }
      try {
        const h = await apiGet<HistoryVersionItem[]>(
          '/merchant/shopMiniProgram/release/historyVersions',
          { authorizerId },
        );
        setRelHistory(Array.isArray(h) ? h : []);
      } catch {
        setRelHistory([]);
      }
      void loadRelease();
    },
    [loadRelease],
  );

  const openRelease = (authorizerId: number, label: string) => {
    setRelTarget({ id: authorizerId, label });
    setRelState(null);
    setRelLogs([]);
    setRelPages([]);
    setRelCategories([]);
    setRelHistory([]);
    setCommitTemplateId(undefined);
    setCommitVersion('');
    setCommitDesc('');
    setAuditItems([{ address: '', categoryKey: '', title: '', tag: '' }]);
    setRevertAppVersion(-1);
    setRelModalVisible(true);
    void refreshRelease(authorizerId);
  };

  const runReleaseAction = async (fn: () => Promise<unknown>, okMsg: string) => {
    if (!relTarget) return;
    setRelBusy(true);
    try {
      await fn();
      Toast.success({ content: okMsg });
      await refreshRelease(relTarget.id);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    } finally {
      setRelBusy(false);
    }
  };

  const doCommit = () => {
    if (!relTarget) return;
    if (commitTemplateId === undefined) {
      Toast.warning({ content: t('请选择代码模板') });
      return;
    }
    void runReleaseAction(
      () =>
        apiPost('/merchant/shopMiniProgram/release/commit', {
          authorizerId: relTarget.id,
          templateId: commitTemplateId,
          userVersion: commitVersion.trim(),
          userDesc: commitDesc.trim(),
        }),
      t('代码已上传，体验版已生成'),
    );
  };

  const doSubmitAudit = () => {
    if (!relTarget) return;
    const itemList = auditItems.map((it) => {
      const cat = relCategories.find(
        (c) => `${c.firstId}_${c.secondId}` === it.categoryKey,
      );
      return {
        address: it.address,
        tag: it.tag,
        title: it.title,
        firstClass: cat?.firstClass || '',
        secondClass: cat?.secondClass || '',
        firstId: cat?.firstId || 0,
        secondId: cat?.secondId || 0,
      };
    });
    void runReleaseAction(
      () =>
        apiPost('/merchant/shopMiniProgram/release/submitAudit', {
          authorizerId: relTarget.id,
          itemList,
        }),
      t('已提交审核'),
    );
  };

  const addDraftToTemplate = async (draft: TemplateDraftItem) => {
    try {
      await apiPost('/merchant/shopMiniProgram/release/templates/add', {
        draftId: draft.draftId,
        templateType: 0,
      });
      Toast.success({ content: t('已加入模板库') });
      void loadTemplates();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const deleteTemplate = async (tpl: CodeTemplateItem) => {
    try {
      await apiPost('/merchant/shopMiniProgram/release/templates/delete', {
        templateId: tpl.templateId,
      });
      Toast.success({ content: t('模板已删除') });
      void loadTemplates();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
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
      width: 250,
      render: (_: unknown, row: AuthItem) => (
        <Space>
          <Button
            theme="borderless"
            size="small"
            disabled={row.status !== 1}
            onClick={() =>
              openRelease(row.id, `${row.shopTitle || row.shopId} · ${row.nickName || row.authorizerAppId}`)
            }
          >
            {t('版本发布')}
          </Button>
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

  const auditTag = (s: VersionStateItem) => {
    switch (s.auditStatus) {
      case 0:
        return <Tag color="green">{t('审核通过')}</Tag>;
      case 1:
        return <Tag color="red">{t('审核被拒')}</Tag>;
      case 2:
        return <Tag color="blue">{t('审核中')}</Tag>;
      case 3:
        return <Tag color="grey">{t('已撤回')}</Tag>;
      case 4:
        return <Tag color="orange">{t('审核延后')}</Tag>;
      default:
        return <Tag color="grey">{t('未提审')}</Tag>;
    }
  };

  const stageText = (stage: number) =>
    [
      t('未上传'),
      t('已上传待提审'),
      t('审核中'),
      t('待发布'),
      t('审核被拒'),
      t('已发布'),
    ][stage] || '--';

  const releaseColumns = [
    {
      title: t('店铺'),
      dataIndex: 'shopTitle',
      width: 180,
      render: (_: unknown, row: VersionStateItem) => (
        <span>
          {row.shopTitle || '--'}
          <Text type="tertiary" size="small">{` #${row.shopId}`}</Text>
        </span>
      ),
    },
    {
      title: t('授权小程序'),
      dataIndex: 'authorizerAppId',
      width: 220,
      render: (_: unknown, row: VersionStateItem) => (
        <span>
          {row.nickName || '--'}
          <br />
          <Text type="tertiary" size="small">
            {row.authorizerAppId}
          </Text>
        </span>
      ),
    },
    {
      title: t('开发版本'),
      dataIndex: 'userVersion',
      width: 180,
      render: (_: unknown, row: VersionStateItem) =>
        row.userVersion ? (
          <span>
            {row.userVersion}
            <br />
            <Text type="tertiary" size="small">
              {row.userDesc || ''}
            </Text>
          </span>
        ) : (
          '--'
        ),
    },
    {
      title: t('阶段'),
      dataIndex: 'stage',
      width: 110,
      render: (v: number) => stageText(v),
    },
    {
      title: t('审核状态'),
      dataIndex: 'auditStatus',
      width: 100,
      render: (_: unknown, row: VersionStateItem) => auditTag(row),
    },
    {
      title: t('线上版本'),
      dataIndex: 'onlineUserVersion',
      width: 120,
      render: (v: string) => v || '--',
    },
    {
      title: t('更新时间'),
      dataIndex: 'updateTime',
      width: 150,
      render: (v: number) => fmtTime(v),
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 110,
      render: (_: unknown, row: VersionStateItem) => (
        <Button
          theme="borderless"
          size="small"
          disabled={row.authStatus !== 1}
          onClick={() =>
            openRelease(
              row.authorizerId,
              `${row.shopTitle || row.shopId} · ${row.nickName || row.authorizerAppId}`,
            )
          }
        >
          {t('发布管理')}
        </Button>
      ),
    },
  ];

  const draftColumns = [
    { title: t('草稿 ID'), dataIndex: 'draftId', width: 90 },
    { title: t('版本'), dataIndex: 'userVersion', width: 110 },
    { title: t('描述'), dataIndex: 'userDesc', render: (v: string) => v || '--' },
    {
      title: t('来源小程序'),
      dataIndex: 'sourceMiniprogram',
      width: 200,
      render: (_: unknown, row: TemplateDraftItem) => (
        <span>
          {row.sourceMiniprogram || '--'}
          <br />
          <Text type="tertiary" size="small">
            {row.sourceMiniprogramAppid}
          </Text>
        </span>
      ),
    },
    {
      title: t('创建时间'),
      dataIndex: 'createTime',
      width: 150,
      render: (v: number) => fmtTime(v),
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 120,
      render: (_: unknown, row: TemplateDraftItem) => (
        <Button
          theme="borderless"
          size="small"
          onClick={() => void addDraftToTemplate(row)}
        >
          {t('加入模板库')}
        </Button>
      ),
    },
  ];

  const templateColumns = [
    { title: t('模板 ID'), dataIndex: 'templateId', width: 110 },
    { title: t('版本'), dataIndex: 'userVersion', width: 110 },
    { title: t('描述'), dataIndex: 'userDesc', render: (v: string) => v || '--' },
    {
      title: t('来源小程序'),
      dataIndex: 'sourceMiniprogram',
      width: 200,
      render: (_: unknown, row: CodeTemplateItem) => (
        <span>
          {row.sourceMiniprogram || '--'}
          <br />
          <Text type="tertiary" size="small">
            {row.sourceMiniprogramAppid}
          </Text>
        </span>
      ),
    },
    {
      title: t('创建时间'),
      dataIndex: 'createTime',
      width: 150,
      render: (v: number) => fmtTime(v),
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 90,
      render: (_: unknown, row: CodeTemplateItem) => (
        <Popconfirm
          title={t('删除该模板？')}
          content={t('删除后不能再用该模板为店铺提交代码，已上传到店铺的代码不受影响。')}
          onConfirm={() => void deleteTemplate(row)}
        >
          <Button theme="borderless" size="small" type="danger">
            {t('删除')}
          </Button>
        </Popconfirm>
      ),
    },
  ];

  const logColumns = [
    {
      title: t('时间'),
      dataIndex: 'addTime',
      width: 140,
      render: (v: number) => fmtTime(v),
    },
    { title: t('动作'), dataIndex: 'action', width: 110 },
    {
      title: t('结果'),
      dataIndex: 'success',
      width: 70,
      render: (v: number) =>
        v === 1 ? (
          <Tag color="green">{t('成功')}</Tag>
        ) : (
          <Tag color="red">{t('失败')}</Tag>
        ),
    },
    {
      title: t('发起方'),
      dataIndex: 'operatorType',
      width: 90,
      render: (v: string) => (v === 'merchant' ? t('商家') : t('平台')),
    },
    { title: t('错误'), dataIndex: 'errMsg', render: (v: string) => v || '--' },
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

      <Space
        style={{ margin: '24px 0 12px', width: '100%', justifyContent: 'space-between' }}
      >
        <div>
          <Title heading={6} style={{ margin: 0 }}>
            {t('模式② · 版本发布（代提审 / 代发布）')}
          </Title>
          <Text type="tertiary" size="small">
            {t(
              '平台用统一代码模板为已授权店铺小程序代提交代码、提交审核与发布，商家也可对本店小程序自助发起。审核结果以微信为准，点「发布管理」里的同步按钮拉取最新状态。',
            )}
          </Text>
        </div>
        <Button onClick={() => void loadRelease()}>{t('刷新')}</Button>
      </Space>
      {tplVisible ? (
        <>
          <Text strong style={{ display: 'block', marginBottom: 8 }}>
            {t('代码模板库（仅平台）')}
          </Text>
          <Text type="tertiary" size="small" style={{ display: 'block', marginBottom: 8 }}>
            {t('草稿来自第三方平台的开发小程序（在微信开发者工具上传到草稿箱）；把草稿加入模板库后，即可为任意已授权店铺按模板提交代码。')}
          </Text>
          <Text style={{ display: 'block', marginBottom: 4 }}>{t('草稿箱')}</Text>
          <Table
            columns={draftColumns}
            dataSource={drafts}
            rowKey="draftId"
            pagination={false}
            style={{ marginBottom: 12 }}
          />
          <Text style={{ display: 'block', marginBottom: 4 }}>{t('模板库')}</Text>
          <Table
            columns={templateColumns}
            dataSource={templates}
            rowKey="templateId"
            pagination={false}
            style={{ marginBottom: 16 }}
          />
        </>
      ) : null}
      <Spin spinning={releaseLoading}>
        <Table
          columns={releaseColumns}
          dataSource={releaseRows}
          rowKey="authorizerId"
          pagination={false}
        />
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

      <Modal
        title={`${t('版本发布')} · ${relTarget?.label || ''}`}
        visible={relModalVisible}
        onCancel={() => setRelModalVisible(false)}
        footer={null}
        width={840}
      >
        <Spin spinning={relBusy}>
          <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
            <Space spacing={16} wrap>
              <Text>
                {t('阶段')}：{relState ? stageText(relState.stage) : '--'}
              </Text>
              <Text>
                {t('开发版本')}：{relState?.userVersion || '--'}
                {relState?.userDesc ? `（${relState.userDesc}）` : ''}
              </Text>
              <Text>
                {t('线上版本')}：{relState?.onlineUserVersion || '--'}
              </Text>
              <Space align="center" spacing={6}>
                <Text>{t('审核状态')}：</Text>
                {relState ? auditTag(relState) : '--'}
                {relState && relState.auditId ? (
                  <Text type="tertiary" size="small">
                    auditid {relState.auditId}
                  </Text>
                ) : null}
              </Space>
            </Space>
            {relState && relState.auditStatus === 1 && relState.auditReason ? (
              <Text type="danger" size="small">
                {t('拒绝原因')}：{relState.auditReason}
              </Text>
            ) : null}

            <Text strong>{t('① 上传代码（生成体验版）')}</Text>
            <Space align="center" spacing={12} wrap>
              <Select
                style={{ width: 300 }}
                placeholder={t('选择代码模板')}
                value={commitTemplateId}
                onChange={(v) => setCommitTemplateId(Number(v))}
                optionList={templates.map((tpl) => ({
                  value: tpl.templateId,
                  label: `${tpl.templateId} · ${tpl.userVersion} ${tpl.userDesc || ''}`,
                }))}
              />
              <Input
                style={{ width: 140 }}
                placeholder={t('版本号，如 1.2.0')}
                value={commitVersion}
                onChange={setCommitVersion}
                showClear
              />
              <Input
                style={{ width: 220 }}
                placeholder={t('版本描述')}
                value={commitDesc}
                onChange={setCommitDesc}
                showClear
              />
              <Button type="primary" theme="solid" onClick={doCommit}>
                {t('上传代码')}
              </Button>
            </Space>
            <Text type="tertiary" size="small">
              {t('上传时会自动按本店生成 ext 配置（店铺归属由后端按小程序 AppID 解析，ext 仅作展示加速）。上传后请等微信代码检测完成再提交审核，过早提交微信会返回 61039。')}
            </Text>

            <Text strong>{t('② 提交审核')}</Text>
            {auditItems.map((it, idx) => (
              <Space key={idx} align="center" spacing={12} wrap>
                <Select
                  style={{ width: 220 }}
                  placeholder={t('页面路径')}
                  value={it.address || undefined}
                  onChange={(v) =>
                    setAuditItems((prev) =>
                      prev.map((p, i) => (i === idx ? { ...p, address: String(v) } : p)),
                    )
                  }
                  optionList={relPages.map((p) => ({ value: p, label: p }))}
                />
                <Select
                  style={{ width: 220 }}
                  placeholder={t('服务类目')}
                  value={it.categoryKey || undefined}
                  onChange={(v) =>
                    setAuditItems((prev) =>
                      prev.map((p, i) =>
                        i === idx ? { ...p, categoryKey: String(v) } : p,
                      ),
                    )
                  }
                  optionList={relCategories.map((c) => ({
                    value: `${c.firstId}_${c.secondId}`,
                    label: `${c.firstClass} / ${c.secondClass}`,
                  }))}
                />
                <Input
                  style={{ width: 140 }}
                  placeholder={t('页面标题')}
                  value={it.title}
                  onChange={(v) =>
                    setAuditItems((prev) =>
                      prev.map((p, i) => (i === idx ? { ...p, title: v } : p)),
                    )
                  }
                />
                <Input
                  style={{ width: 120 }}
                  placeholder={t('类目标签')}
                  value={it.tag}
                  onChange={(v) =>
                    setAuditItems((prev) =>
                      prev.map((p, i) => (i === idx ? { ...p, tag: v } : p)),
                    )
                  }
                />
                {auditItems.length > 1 ? (
                  <Button
                    theme="borderless"
                    size="small"
                    type="danger"
                    onClick={() =>
                      setAuditItems((prev) => prev.filter((_, i) => i !== idx))
                    }
                  >
                    {t('移除')}
                  </Button>
                ) : null}
              </Space>
            ))}
            <Space>
              {auditItems.length < 5 ? (
                <Button
                  size="small"
                  onClick={() =>
                    setAuditItems((prev) => [
                      ...prev,
                      { address: '', categoryKey: '', title: '', tag: '' },
                    ])
                  }
                >
                  {t('添加提审项')}
                </Button>
              ) : null}
              <Button type="primary" theme="solid" onClick={doSubmitAudit}>
                {t('提交审核')}
              </Button>
            </Space>

            <Text strong>{t('③ 审核与发布')}</Text>
            <Space spacing={12} wrap>
              <Button
                onClick={() =>
                  void runReleaseAction(
                    () =>
                      apiPost('/merchant/shopMiniProgram/release/syncAudit', {
                        authorizerId: relTarget?.id,
                      }),
                    t('审核状态已同步'),
                  )
                }
              >
                {t('同步审核状态')}
              </Button>
              <Popconfirm
                title={t('撤回当前审核单？')}
                content={t('撤回后可修改代码重新提审；微信对撤回次数有限制。')}
                onConfirm={() =>
                  void runReleaseAction(
                    () =>
                      apiPost('/merchant/shopMiniProgram/release/withdrawAudit', {
                        authorizerId: relTarget?.id,
                      }),
                    t('审核已撤回'),
                  )
                }
              >
                <Button disabled={!relState || (relState.auditStatus !== 2 && relState.auditStatus !== 4)}>
                  {t('撤回审核')}
                </Button>
              </Popconfirm>
              <Popconfirm
                title={t('发布当前审核通过的版本？')}
                content={t('发布后该版本立即成为线上版本，买家打开小程序即见新版本。')}
                onConfirm={() =>
                  void runReleaseAction(
                    () =>
                      apiPost('/merchant/shopMiniProgram/release/release', {
                        authorizerId: relTarget?.id,
                      }),
                    t('已发布'),
                  )
                }
              >
                <Button
                  type="primary"
                  theme="solid"
                  disabled={!relState || relState.auditStatus !== 0}
                >
                  {t('发布')}
                </Button>
              </Popconfirm>
            </Space>

            <Text strong>{t('④ 版本回退')}</Text>
            <Space align="center" spacing={12} wrap>
              <Select
                style={{ width: 300 }}
                value={revertAppVersion}
                onChange={(v) => setRevertAppVersion(Number(v))}
                optionList={[
                  { value: -1, label: t('回退到上一个版本') },
                  ...relHistory.map((h) => ({
                    value: h.appVersion,
                    label: `${h.userVersion}（${t('版本')} ${h.appVersion}）${h.userDesc ? ` · ${h.userDesc}` : ''}`,
                  })),
                ]}
              />
              <Popconfirm
                title={t('回退线上版本？')}
                content={t('回退过的版本不能再次回退；没有上一个线上版本时微信会拒绝。')}
                onConfirm={() =>
                  void runReleaseAction(
                    () =>
                      apiPost('/merchant/shopMiniProgram/release/revert', {
                        authorizerId: relTarget?.id,
                        ...(revertAppVersion > 0
                          ? { appVersion: revertAppVersion }
                          : {}),
                      }),
                    t('已回退'),
                  )
                }
              >
                <Button type="danger">{t('执行回退')}</Button>
              </Popconfirm>
            </Space>

            <Text strong>{t('操作流水')}</Text>
            <Table
              columns={logColumns}
              dataSource={relLogs}
              rowKey="id"
              pagination={false}
              style={{ width: '100%' }}
            />
          </Space>
        </Spin>
      </Modal>
    </div>
  );
}
