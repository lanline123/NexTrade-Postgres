/**
 * AI 运营 MCP 设置（一级 AI → 二级 MCP 管理 → 三级「商品 MCP 设置」）。
 *
 * 所有开关都落库到 config.aiMcpSettings；MCP 总开关关闭时 /adminapi/ai/mcp 直接 403。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, DatePicker, Input, InputNumber, Switch, Tag, Toast, Spin } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

/** 旧版 views/setting/config/base/src/workbuddySettings.ts 的掩码规则。 */
function maskKey(value: string): string {
  if (!value) return '';
  if (value.length <= 8) return '*'.repeat(value.length);
  return `${value.slice(0, 3)}${'*'.repeat(Math.max(4, value.length - 7))}${value.slice(-4)}`;
}

interface WorkBuddySettings {
  configured: boolean;
  maskedKey: string;
  expiresAt?: number | null;
}

interface McpSettings {
  enabled: boolean;
  token: string;
  productEnabled: boolean;
  decorateEnabled: boolean;
  analyticsEnabled: boolean;
  orderEnabled: boolean;
  memberEnabled: boolean;
  contentEnabled: boolean;
  priceChangePct: number;
  priceChangeAmount: number;
  stockChangeCap: number;
  requireApprovalPrice: boolean;
  requireApprovalPublish: boolean;
  requireApprovalBatch: boolean;
  requireApprovalImport: boolean;
  endpoint?: string;
}

const DEFAULTS: McpSettings = {
  enabled: false,
  token: '',
  productEnabled: true,
  decorateEnabled: false,
  analyticsEnabled: true,
  orderEnabled: false,
  memberEnabled: false,
  contentEnabled: false,
  priceChangePct: 10,
  priceChangeAmount: 1000,
  stockChangeCap: 1000,
  requireApprovalPrice: true,
  requireApprovalPublish: true,
  requireApprovalBatch: true,
  requireApprovalImport: true,
};

const sectionStyle: React.CSSProperties = {
  background: '#fff',
  borderRadius: 8,
  padding: '16px 20px',
  marginBottom: 16,
};
const rowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 12,
  padding: '10px 0',
  borderBottom: '1px solid #f0f1f2',
};
const labelStyle: React.CSSProperties = { width: 180, color: '#444', flex: '0 0 auto' };
const hintStyle: React.CSSProperties = { color: '#8a8f99', fontSize: 12 };

export default function AiMcpSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [settings, setSettings] = useState<McpSettings>(DEFAULTS);
  const [wb, setWb] = useState<WorkBuddySettings>({ configured: false, maskedKey: '' });
  const [wbExpiry, setWbExpiry] = useState<Date | null>(null);
  const [wbPendingKey, setWbPendingKey] = useState('');
  const [wbSaving, setWbSaving] = useState(false);
  const [wbGenerating, setWbGenerating] = useState(false);

  const load = useCallback(async () => {
    try {
      const data = await apiGet<McpSettings>('/ai/mcp/settings');
      setSettings({ ...DEFAULTS, ...data });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载 MCP 设置失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  const loadWorkBuddy = useCallback(async () => {
    try {
      const data = await apiGet<WorkBuddySettings>('/setting/config/workbuddySettings');
      setWb({
        configured: Boolean(data?.configured),
        maskedKey: data?.maskedKey || '',
        expiresAt: data?.expiresAt || null,
      });
      setWbExpiry(data?.expiresAt ? new Date(data.expiresAt * 1000) : null);
      setWbPendingKey('');
    } catch {
      // 商品导入 MCP 是独立能力，加载失败不影响上方 AI MCP 设置
    }
  }, []);

  useEffect(() => {
    void load();
    void loadWorkBuddy();
  }, [load, loadWorkBuddy]);

  const patch = (next: Partial<McpSettings>) => setSettings((prev) => ({ ...prev, ...next }));

  const save = async () => {
    setSaving(true);
    try {
      const res = await apiPost<{ saved: boolean; token: string }>('/ai/mcp/settings', settings);
      if (res?.token) patch({ token: res.token });
      Toast.success({ content: '已保存，MCP 配置立即生效' });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '保存失败' });
    } finally {
      setSaving(false);
    }
  };

  const regenerate = async () => {
    try {
      const res = await apiPost<{ token: string }>('/ai/mcp/settings/token');
      patch({ token: res.token });
      Toast.success({ content: '已生成新 Token，旧 Token 立即失效' });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '生成失败' });
    }
  };

  const copy = (text: string) => {
    if (!text) return;
    void navigator.clipboard?.writeText(text).then(
      () => Toast.success({ content: '已复制' }),
      () => Toast.error({ content: '复制失败，请手动选择复制' }),
    );
  };

  const generateWbKey = async () => {
    setWbGenerating(true);
    try {
      const res = await apiPost<{ key?: string }>('/setting/config/generateWorkbuddyKey');
      if (!res?.key) throw new Error('未返回密钥');
      setWbPendingKey(res.key);
      Toast.success({ content: '新密钥已生成，请设置有效期后保存' });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '生成失败' });
    } finally {
      setWbGenerating(false);
    }
  };

  const copyWbKey = async () => {
    let key = wbPendingKey;
    if (!key) {
      try {
        const res = await apiPost<{ key?: string }>('/setting/config/revealWorkbuddyKey');
        key = res?.key || '';
      } catch (e) {
        Toast.error({ content: (e as Error).message || '读取密钥失败' });
        return;
      }
    }
    if (!key) {
      Toast.error({ content: '当前没有可复制的密钥，请先生成' });
      return;
    }
    copy(key);
  };

  const saveWorkBuddy = async () => {
    if (!wbPendingKey && !wb.configured) {
      Toast.error({ content: '请先生成密钥' });
      return;
    }
    if (!wbExpiry || !Number.isFinite(wbExpiry.getTime())) {
      Toast.error({ content: '请设置密钥有效期' });
      return;
    }
    if (wbExpiry.getTime() <= Date.now()) {
      Toast.error({ content: '有效期必须晚于当前时间' });
      return;
    }
    setWbSaving(true);
    try {
      const payload: Record<string, unknown> = {
        workbuddyApiKeyExpiresAt: Math.floor(wbExpiry.getTime() / 1000),
      };
      if (wbPendingKey) payload.workbuddyApiKey = wbPendingKey;
      await apiPost('/setting/config/saveGlobal', payload);
      Toast.success({ content: '商品导入 MCP 密钥已保存' });
      await loadWorkBuddy();
    } catch (e) {
      Toast.error({ content: (e as Error).message || '保存失败' });
    } finally {
      setWbSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin />
      </div>
    );
  }

  const endpoint = settings.endpoint || '/adminapi/ai/mcp';
  const endpointUrl = `${window.location.origin}${endpoint}`;
  const wbEndpoint = `${window.location.origin}/adminapi/openapi/mcp/v1`;
  const wbKeyPreview = wbPendingKey ? maskKey(wbPendingKey) : wb.maskedKey;

  return (
    <div style={{ maxWidth: 980 }}>
      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>MCP 服务</h3>
        <div style={rowStyle}>
          <span style={labelStyle}>启用 MCP</span>
          <Switch checked={settings.enabled} onChange={(v) => patch({ enabled: v })} />
          <span style={hintStyle}>关闭时 /adminapi/ai/mcp 直接拒绝访问</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>MCP 地址</span>
          <Input value={endpointUrl} readOnly style={{ flex: 1 }} />
          <Button onClick={() => copy(endpointUrl)}>复制</Button>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>访问 Token</span>
          <Input value={settings.token} readOnly style={{ flex: 1 }} />
          <Button onClick={() => copy(settings.token)}>复制</Button>
          <Button onClick={() => void regenerate()}>重新生成</Button>
        </div>
        <p style={hintStyle}>
          客户端请求头：<code>Authorization: Bearer &lt;Token&gt;</code>；MCP 客户端填上面地址即可。
        </p>
      </div>

      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>权限范围</h3>
        <div style={rowStyle}>
          <span style={labelStyle}>商品管理</span>
          <Switch checked={settings.productEnabled} onChange={(v) => patch({ productEnabled: v })} />
          <span style={hintStyle}>搜索/详情/字段/价格/库存/上下架/批量操作/商品导入</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>装修管理</span>
          <Switch checked={settings.decorateEnabled} onChange={(v) => patch({ decorateEnabled: v })} />
          <span style={hintStyle}>装修页列表/详情/保存草稿/发布</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>报表分析（只读）</span>
          <Switch checked={settings.analyticsEnabled} onChange={(v) => patch({ analyticsEnabled: v })} />
          <span style={hintStyle}>销售汇总、销量 Top、库存预警</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>订单管理</span>
          <Switch checked={settings.orderEnabled} onChange={(v) => patch({ orderEnabled: v })} />
          <span style={hintStyle}>订单查询/详情、后台备注（不含退款/取消）</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>会员查询（只读）</span>
          <Switch checked={settings.memberEnabled} onChange={(v) => patch({ memberEnabled: v })} />
          <span style={hintStyle}>会员搜索、会员详情</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>内容查询（只读）</span>
          <Switch checked={settings.contentEnabled} onChange={(v) => patch({ contentEnabled: v })} />
          <span style={hintStyle}>文章/公告列表与详情</span>
        </div>
      </div>

      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>风控阈值</h3>
        <div style={rowStyle}>
          <span style={labelStyle}>改价幅度阈值（%）</span>
          <InputNumber
            value={settings.priceChangePct}
            min={0}
            onChange={(v) => patch({ priceChangePct: Number(v) || 0 })}
          />
          <span style={hintStyle}>0 表示不限制；超过则转审批</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>改价金额阈值</span>
          <InputNumber
            value={settings.priceChangeAmount}
            min={0}
            onChange={(v) => patch({ priceChangeAmount: Number(v) || 0 })}
          />
          <span style={hintStyle}>单次价格变动金额上限</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>库存单次变更上限</span>
          <InputNumber
            value={settings.stockChangeCap}
            min={0}
            onChange={(v) => patch({ stockChangeCap: Number(v) || 0 })}
          />
          <span style={hintStyle}>0 表示不限制；超过则转审批</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>改价需审批</span>
          <Switch
            checked={settings.requireApprovalPrice}
            onChange={(v) => patch({ requireApprovalPrice: v })}
          />
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>发布装修需审批</span>
          <Switch
            checked={settings.requireApprovalPublish}
            onChange={(v) => patch({ requireApprovalPublish: v })}
          />
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>批量回收站/删除需审批</span>
          <Switch
            checked={settings.requireApprovalBatch}
            onChange={(v) => patch({ requireApprovalBatch: v })}
          />
          <span style={hintStyle}>商品批量移入回收站、彻底删除前需审批</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>商品导入需审批</span>
          <Switch
            checked={settings.requireApprovalImport}
            onChange={(v) => patch({ requireApprovalImport: v })}
          />
          <span style={hintStyle}>批量创建商品前需审批</span>
        </div>
      </div>

      <Button theme="solid" type="primary" loading={saving} onClick={() => void save()}>
        保存设置
      </Button>

      <div style={{ ...sectionStyle, marginTop: 16 }}>
        <h3 style={{ margin: '0 0 8px' }}>商品导入 MCP（WorkBuddy）</h3>
        <p style={hintStyle}>
          供 WorkBuddy 商品导入/管理开放接口使用，请求头 <code>X-MCP-Api-Key</code> 校验；
          与上方 AI 运营 MCP 的 Token 相互独立。
        </p>
        <div style={rowStyle}>
          <span style={labelStyle}>MCP 密钥</span>
          <Input value={wbKeyPreview} readOnly placeholder="尚未配置" style={{ flex: 1 }} />
          {wbPendingKey && <Tag color="orange">未保存</Tag>}
          <Button loading={wbGenerating} onClick={() => void generateWbKey()}>
            生成密钥
          </Button>
          <Button disabled={!wbPendingKey && !wb.configured} onClick={() => void copyWbKey()}>
            复制密钥
          </Button>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>密钥有效期</span>
          <DatePicker
            type="dateTime"
            value={wbExpiry ?? undefined}
            onChange={(next) => setWbExpiry((next as Date) || null)}
            style={{ width: 260 }}
          />
          <span style={hintStyle}>到期后 MCP 请求会被拒绝</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>接口前缀</span>
          <Input value={wbEndpoint} readOnly style={{ flex: 1 }} />
          <Button onClick={() => copy(wbEndpoint)}>复制</Button>
        </div>
        {wbPendingKey && (
          <p style={{ ...hintStyle, color: '#d2691e' }}>
            新密钥尚未保存，点击下方按钮后旧密钥才会失效。
          </p>
        )}
        <Button
          theme="solid"
          type="primary"
          loading={wbSaving}
          onClick={() => void saveWorkBuddy()}
          style={{ marginTop: 8 }}
        >
          保存密钥配置
        </Button>
      </div>
    </div>
  );
}
