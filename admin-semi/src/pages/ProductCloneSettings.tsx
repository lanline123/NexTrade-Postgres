import { useEffect, useState } from 'react';
import { Button, Card, Input, InputNumber, Radio, RadioGroup, Select, Space, Spin, Switch, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

const { Text, Title } = Typography;

/**
 * 商品复刻设置（设置 → 商城设置 → 商品复刻设置）。
 *
 * 配置存后端 config code `productCloneSettings`。密钥（万邦 key/secret、
 * 边车 token）只写不回显：读取只回「是否已配置 + 尾 4 位」，保存时留空 = 不改。
 * 边车各平台 cookie 状态来自通道健康检查（边车 /healthz），只读展示。
 */

interface CloneSettingsVO {
  defaultChannel: 'api' | 'selfhost';
  defaultChannelByPlatform: Record<string, string>;
  apiEnabled: boolean;
  apiKeySet: boolean;
  apiKeyTail: string;
  apiSecretSet: boolean;
  apiPddRecharged: boolean;
  selfhostEnabled: boolean;
  selfhostEndpoint: string;
  selfhostTokenSet: boolean;
  fallbackBToA: boolean;
  dailyApiBudgetFen: number | null;
}

interface ChannelsInfo {
  selfhost: { health: { alive: boolean; platforms: Record<string, { cookie: string }> } };
}

const PLATFORMS: Array<{ key: string; label: string }> = [
  { key: 'taobao', label: '淘宝' },
  { key: 'tmall', label: '天猫' },
  { key: 'jd', label: '京东' },
  { key: 'pdd', label: '拼多多' },
  { key: '1688', label: '1688' },
  { key: 'amazon', label: '亚马逊' },
  { key: 'meituan', label: '美团（不支持）' },
];

export default function ProductCloneSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [vo, setVo] = useState<CloneSettingsVO | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [apiSecret, setApiSecret] = useState('');
  const [selfhostToken, setSelfhostToken] = useState('');
  const [budgetYuan, setBudgetYuan] = useState<number | undefined>(undefined);
  const [cookieStates, setCookieStates] = useState<Record<string, { cookie: string }>>({});
  const [sidecarAlive, setSidecarAlive] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const s = await apiGet<CloneSettingsVO>('/setting/config/productCloneSettings');
      setVo(s);
      setBudgetYuan(s.dailyApiBudgetFen ? s.dailyApiBudgetFen / 100 : undefined);
      try {
        const info = await apiGet<ChannelsInfo>('/product/productClone/channels');
        setCookieStates(info.selfhost?.health?.platforms || {});
        setSidecarAlive(!!info.selfhost?.health?.alive);
      } catch { /* 边车未部署时忽略 */ }
    } catch (e) {
      Toast.error(`加载设置失败：${(e as Error).message}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  if (loading || !vo) return <Spin style={{ display: 'block', margin: '80px auto' }} />;

  const set = (patch: Partial<CloneSettingsVO>) => setVo({ ...vo, ...patch });

  const save = async () => {
    setSaving(true);
    try {
      const payload: Record<string, unknown> = {
        defaultChannel: vo.defaultChannel,
        defaultChannelByPlatform: vo.defaultChannelByPlatform,
        apiEnabled: vo.apiEnabled,
        apiPddRecharged: vo.apiPddRecharged,
        selfhostEnabled: vo.selfhostEnabled,
        selfhostEndpoint: vo.selfhostEndpoint,
        fallbackBToA: vo.fallbackBToA,
        dailyApiBudgetFen: budgetYuan ? Math.round(budgetYuan * 100) : null,
      };
      if (apiKey.trim()) payload.apiKey = apiKey.trim();
      if (apiSecret.trim()) payload.apiSecret = apiSecret.trim();
      if (selfhostToken.trim()) payload.selfhostToken = selfhostToken.trim();
      const saved = await apiPost<CloneSettingsVO>('/setting/config/saveProductCloneSettings', payload);
      setVo(saved);
      setApiKey(''); setApiSecret(''); setSelfhostToken('');
      Toast.success('已保存');
    } catch (e) {
      Toast.error(`保存失败：${(e as Error).message}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ padding: 16, maxWidth: 860 }}>
      <Title heading={4} style={{ marginBottom: 4 }}>商品复刻设置</Title>
      <Text type="tertiary">双通道的全局默认、凭证与费用上限。复刻页每次打开按这里的默认值预选通道，单次可在页面上切换。</Text>

      <Card style={{ marginTop: 16 }} bodyStyle={{ padding: 16 }}>
        <Text strong>默认通道</Text>
        <div style={{ marginTop: 8 }}>
          <RadioGroup value={vo.defaultChannel} onChange={(e) => set({ defaultChannel: e.target.value })}>
            <Radio value="api">数据接口（通道 A）</Radio>
            <Radio value="selfhost">自建抓取（通道 B）</Radio>
          </RadioGroup>
        </div>
        <div style={{ marginTop: 12 }}>
          <Text type="tertiary">分平台覆盖（不设则跟随全局默认）：</Text>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 12, marginTop: 8 }}>
            {PLATFORMS.map((p) => (
              <div key={p.key}>
                <Text style={{ marginRight: 6 }}>{p.label}</Text>
                <Select
                  value={vo.defaultChannelByPlatform[p.key] || ''}
                  onChange={(v) => {
                    const next = { ...vo.defaultChannelByPlatform };
                    if (v) next[p.key] = String(v); else delete next[p.key];
                    set({ defaultChannelByPlatform: next });
                  }}
                  style={{ width: 130 }}
                >
                  <Select.Option value="">跟随全局</Select.Option>
                  <Select.Option value="api">数据接口</Select.Option>
                  <Select.Option value="selfhost">自建抓取</Select.Option>
                </Select>
              </div>
            ))}
          </div>
        </div>
        <div style={{ marginTop: 14 }}>
          <Space>
            <Switch checked={vo.fallbackBToA} onChange={(v) => set({ fallbackBToA: v })} />
            <Text>自建抓取失败时自动转数据接口重试一次</Text>
            <Text type="tertiary">（默认关：自动转会产生接口费用；只提供 B→A 单向，不提供反向）</Text>
          </Space>
        </div>
      </Card>

      <Card style={{ marginTop: 16 }} bodyStyle={{ padding: 16 }}>
        <Space>
          <Text strong>通道 A · 数据接口（万邦 onebound）</Text>
          <Switch checked={vo.apiEnabled} onChange={(v) => set({ apiEnabled: v })} />
          <Text type="tertiary">{vo.apiEnabled ? '已开启' : '已关闭'}</Text>
        </Space>
        <div style={{ display: 'flex', gap: 16, marginTop: 12, flexWrap: 'wrap' }}>
          <div>
            <Text>API Key {vo.apiKeySet && <Tag size="small" color="green">已配置 ···{vo.apiKeyTail}</Tag>}</Text>
            <Input
              mode="password"
              placeholder={vo.apiKeySet ? '留空表示不修改' : '请填写万邦 key'}
              value={apiKey}
              onChange={setApiKey}
              style={{ width: 280, display: 'block', marginTop: 4 }}
            />
          </div>
          <div>
            <Text>API Secret {vo.apiSecretSet && <Tag size="small" color="green">已配置</Tag>}</Text>
            <Input
              mode="password"
              placeholder={vo.apiSecretSet ? '留空表示不修改' : '请填写万邦 secret'}
              value={apiSecret}
              onChange={setApiSecret}
              style={{ width: 280, display: 'block', marginTop: 4 }}
            />
          </div>
          <div>
            <Text>日费用上限（元）</Text>
            <InputNumber
              placeholder="不限"
              value={budgetYuan}
              onChange={(v) => setBudgetYuan(typeof v === 'number' ? v : undefined)}
              min={0}
              style={{ width: 160, display: 'block', marginTop: 4 }}
            />
            <Text type="tertiary" style={{ fontSize: 12 }}>按预估单价累计，超限后通道 A 任务直接失败，防 key 泄露被刷</Text>
          </div>
        </div>
        <div style={{ marginTop: 12 }}>
          <Space>
            <Switch checked={vo.apiPddRecharged} onChange={(v) => set({ apiPddRecharged: v })} />
            <Text>拼多多接口已在万邦单独充值</Text>
            <Text type="tertiary">（万邦拼多多单独计费；未充值时拼多多解析失败属正常，打开此项后复刻页不再提示）</Text>
          </Space>
        </div>
        <Text type="tertiary" style={{ display: 'block', marginTop: 8, fontSize: 12 }}>
          参考单价：淘宝/1688 约 0.02 元/次，京东高级接口 0.05–0.15 元/次，拼多多单独充值计费。费用按预估口径记入复刻记录，可与万邦账单对账。
        </Text>
      </Card>

      <Card style={{ marginTop: 16 }} bodyStyle={{ padding: 16 }}>
        <Space>
          <Text strong>通道 B · 自建抓取边车</Text>
          <Switch checked={vo.selfhostEnabled} onChange={(v) => set({ selfhostEnabled: v })} />
          <Text type="tertiary">{vo.selfhostEnabled ? '已开启' : '已关闭（一期仅淘宝/天猫）'}</Text>
          {vo.selfhostEnabled && (
            <Tag color={sidecarAlive ? 'green' : 'red'}>{sidecarAlive ? '边车在线' : '边车离线/未部署'}</Tag>
          )}
        </Space>
        <div style={{ display: 'flex', gap: 16, marginTop: 12, flexWrap: 'wrap' }}>
          <div>
            <Text>边车地址</Text>
            <Input
              placeholder="http://127.0.0.1:8791"
              value={vo.selfhostEndpoint}
              onChange={(v) => set({ selfhostEndpoint: v })}
              style={{ width: 280, display: 'block', marginTop: 4 }}
            />
          </div>
          <div>
            <Text>边车 Token {vo.selfhostTokenSet && <Tag size="small" color="green">已配置</Tag>}</Text>
            <Input
              mode="password"
              placeholder={vo.selfhostTokenSet ? '留空表示不修改' : '与边车 SCRAPER_TOKEN 一致'}
              value={selfhostToken}
              onChange={setSelfhostToken}
              style={{ width: 280, display: 'block', marginTop: 4 }}
            />
          </div>
        </div>
        <div style={{ marginTop: 12 }}>
          <Text type="tertiary">平台账号状态（来自边车健康检查，只读）：</Text>
          <div style={{ marginTop: 6 }}>
            <Space wrap>
              {Object.keys(cookieStates).length === 0 && <Text type="tertiary">暂无数据（边车未启用或离线）</Text>}
              {Object.entries(cookieStates).map(([p, st]) => (
                <Tag key={p} color={st.cookie === 'ok' ? 'green' : 'red'}>
                  {p === 'taobao' ? '淘宝' : p === 'tmall' ? '天猫' : p} cookie：{st.cookie === 'ok' ? '已配置' : '缺失'}
                </Tag>
              ))}
            </Space>
          </div>
          <Text type="tertiary" style={{ display: 'block', marginTop: 8, fontSize: 12 }}>
            更新 cookie：用专用小号登录淘宝网页版，复制整串 Cookie 到边车服务器的 cookie 文件后重启边车（见边车 README）。cookie 是否过期需真实解析一次才知道，失败原因会提示登录失效。
          </Text>
        </div>
      </Card>

      <div style={{ marginTop: 16 }}>
        <Button theme="solid" type="primary" loading={saving} onClick={save}>保存设置</Button>
        <Button style={{ marginLeft: 8 }} onClick={load}>重新加载</Button>
      </div>
    </div>
  );
}
