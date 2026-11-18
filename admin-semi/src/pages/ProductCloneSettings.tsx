import { useEffect, useState } from 'react';
import { Button, Card, Input, Space, Spin, Switch, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

const { Text, Title } = Typography;

/**
 * 商品复刻设置（设置 → 商城设置 → 商品复刻设置）。
 *
 * 配置存后端 config code `productCloneSettings`。取数只走自建抓取边车
 * （通道 A 数据接口已于 2026-11-18 整体移除）。边车 token 只写不回显：
 * 读取只回「是否已配置」，保存时留空 = 不改。
 * 边车各平台 cookie 状态来自通道健康检查（边车 /healthz），只读展示。
 */

interface CloneSettingsVO {
  selfhostEnabled: boolean;
  selfhostEndpoint: string;
  selfhostTokenSet: boolean;
}

interface ChannelsInfo {
  selfhost: { health: { alive: boolean; platforms: Record<string, { cookie: string }> } };
}

export default function ProductCloneSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [vo, setVo] = useState<CloneSettingsVO | null>(null);
  const [selfhostToken, setSelfhostToken] = useState('');
  const [cookieStates, setCookieStates] = useState<Record<string, { cookie: string }>>({});
  const [sidecarAlive, setSidecarAlive] = useState(false);

  const load = async () => {
    setLoading(true);
    try {
      const s = await apiGet<CloneSettingsVO>('/setting/config/productCloneSettings');
      setVo(s);
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
        selfhostEnabled: vo.selfhostEnabled,
        selfhostEndpoint: vo.selfhostEndpoint,
      };
      if (selfhostToken.trim()) payload.selfhostToken = selfhostToken.trim();
      const saved = await apiPost<CloneSettingsVO>('/setting/config/saveProductCloneSettings', payload);
      setVo(saved);
      setSelfhostToken('');
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
      <Text type="tertiary">自建抓取边车的启用、地址与凭证。边车一期仅支持淘宝/天猫，其余平台暂不支持复刻。</Text>

      <Card style={{ marginTop: 16 }} bodyStyle={{ padding: 16 }}>
        <Space>
          <Text strong>自建抓取边车</Text>
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
