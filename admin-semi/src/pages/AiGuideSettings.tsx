/**
 * AI 导购配置（一级 AI → 二级 AI 助手 → 三级「导购配置」，我方新增）。
 *
 * 导购与运营助手共用同一 AI 底座（渠道/密钥/用量台账），本页只配导购
 * 引擎真实消费的旋钮：总开关 + 三级 Token 预算，存 config.aiGuideSettings
 * （GET/POST /ai/guide/settings）。引擎每回合现读配置、无缓存，保存后
 * 买家下一回合即按新值执行。
 * 渠道/模型跟随平台默认渠道（在「渠道配置」中设置），本页不单独指定；
 * 引擎没有欢迎语下发机制，本页不设欢迎语字段。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, InputNumber, Spin, Switch, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

interface GuideSettings {
  enabled: boolean;
  sessionTokens: number;
  userDailyTokens: number;
  globalDailyTokens: number;
}

/** 与后端引擎内置默认逐字一致（config 无行时的生效值）。 */
const DEFAULTS: GuideSettings = {
  enabled: true,
  sessionTokens: 60000,
  userDailyTokens: 200000,
  globalDailyTokens: 5000000,
};

const sectionStyle: React.CSSProperties = {
  background: '#fff',
  border: '1px solid #e4e7ec',
  borderRadius: 0,
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

const BUDGET_FIELDS = [
  {
    key: 'sessionTokens',
    label: '单会话 Token 预算',
    hint: '单个导购会话累计（输入 + 输出）上限；达到后提示买家开启新会话继续咨询',
  },
  {
    key: 'userDailyTokens',
    label: '单用户每日 Token 预算',
    hint: '单个买家按北京时间自然日累计上限；达到后当日不再为其调用模型',
  },
  {
    key: 'globalDailyTokens',
    label: '全局每日 Token 预算',
    hint: '全平台买家按北京时间自然日合计上限；达到后当日导购整体熔断',
  },
] as const;

export default function AiGuideSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [settings, setSettings] = useState<GuideSettings>(DEFAULTS);

  const load = useCallback(async () => {
    try {
      const data = await apiGet<GuideSettings>('/ai/guide/settings');
      setSettings({ ...DEFAULTS, ...data });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载导购配置失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const patch = (next: Partial<GuideSettings>) => setSettings((prev) => ({ ...prev, ...next }));

  const save = async () => {
    // 前端先行校验（后端仍会严格校验并指明字段）：预算必须为正整数——
    // 填 0 不会生效（后端引擎将 0 视为未配置、回退默认值）。
    for (const field of BUDGET_FIELDS) {
      const value = settings[field.key];
      if (!Number.isInteger(value) || value <= 0) {
        Toast.error({ content: `${field.label}必须为正整数` });
        return;
      }
    }
    setSaving(true);
    try {
      const saved = await apiPost<GuideSettings>('/ai/guide/settings', settings);
      setSettings({ ...DEFAULTS, ...saved });
      Toast.success({ content: '已保存，导购配置立即生效' });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '保存失败' });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin />
      </div>
    );
  }

  return (
    <div>
      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>导购开关</h3>
        <div style={rowStyle}>
          <span style={labelStyle}>启用导购</span>
          <Switch checked={settings.enabled} onChange={(v) => patch({ enabled: v })} />
          <span style={hintStyle}>
            关闭时买家提问将收到「导购功能暂未开放，敬请期待。」的回复，不调用模型、不消耗预算；历史会话仍可查看
          </span>
        </div>
        <p style={hintStyle}>
          渠道与模型：导购跟随平台默认渠道与其对话模型，在「渠道配置」中设置默认渠道即对导购生效，本页不单独指定。
          未保存过时使用内置默认：开启，预算 60000 / 200000 / 5000000。
        </p>
      </div>

      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>用量预算（Token）</h3>
        {BUDGET_FIELDS.map((field) => (
          <div style={rowStyle} key={field.key}>
            <span style={labelStyle}>{field.label}</span>
            <InputNumber
              value={settings[field.key]}
              min={1}
              step={1000}
              style={{ width: 220 }}
              onChange={(v) => patch({ [field.key]: Number(v) || 0 } as Partial<GuideSettings>)}
            />
            <span style={hintStyle}>{field.hint}</span>
          </div>
        ))}
        <p style={hintStyle}>预算必须为正整数；如需停用导购请关闭上方总开关（预算填 0 不会生效）。</p>
      </div>

      <Button theme="solid" type="primary" loading={saving} onClick={() => void save()}>
        保存设置
      </Button>
    </div>
  );
}
