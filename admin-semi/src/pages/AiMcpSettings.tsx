/**
 * AI 运营 MCP 设置（一级 AI → 二级 MCP 管理 → 三级「商品 MCP 设置」）。
 *
 * 所有开关都落库到 config.aiMcpSettings；MCP 总开关关闭时 /adminapi/ai/mcp 直接 403。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, Input, InputNumber, Switch, Toast, Spin } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

interface McpSettings {
  enabled: boolean;
  token: string;
  productEnabled: boolean;
  decorateEnabled: boolean;
  priceChangePct: number;
  priceChangeAmount: number;
  stockChangeCap: number;
  requireApprovalPrice: boolean;
  requireApprovalPublish: boolean;
  endpoint?: string;
}

const DEFAULTS: McpSettings = {
  enabled: false,
  token: '',
  productEnabled: true,
  decorateEnabled: false,
  priceChangePct: 10,
  priceChangeAmount: 1000,
  stockChangeCap: 1000,
  requireApprovalPrice: true,
  requireApprovalPublish: true,
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

  useEffect(() => {
    void load();
  }, [load]);

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

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin />
      </div>
    );
  }

  const endpoint = settings.endpoint || '/adminapi/ai/mcp';
  const endpointUrl = `${window.location.origin}${endpoint}`;

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
          <span style={hintStyle}>搜索/详情/字段/价格/库存/上下架</span>
        </div>
        <div style={rowStyle}>
          <span style={labelStyle}>装修管理</span>
          <Switch checked={settings.decorateEnabled} onChange={(v) => patch({ decorateEnabled: v })} />
          <span style={hintStyle}>装修页列表/详情/保存草稿/发布</span>
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
      </div>

      <Button theme="solid" type="primary" loading={saving} onClick={() => void save()}>
        保存设置
      </Button>
    </div>
  );
}
