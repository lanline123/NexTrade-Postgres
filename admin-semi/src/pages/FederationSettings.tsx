// 我方新增（平台联邦 P1）：平台联邦设置页（设置 → 商城设置 → 平台联邦）。
import { useEffect, useState } from 'react';
import { Button, Card, Checkbox, Input, Space, Spin, Switch, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import {
  cacheFederationSettings,
  normalizeFederationSettings,
  type FederationSettings as FederationSettingsValue,
} from '../federation';

const { Text, Title } = Typography;

/**
 * 平台联邦设置（我方新增，原版无）：配置存后端 config `federationSettings`，
 * 后台读 GET /setting/config/federationSettings、写 POST /setting/config/saveFederationSettings
 * （整块保存）。本页不受总开关门控常驻——总开关关时正是从这里重新打开。
 * 保存 payload 必须带回 instanceUuid（从加载值原样回传，避免后端重生成）。
 */
export default function FederationSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [settings, setSettings] = useState<FederationSettingsValue>(() => normalizeFederationSettings(null));

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<Record<string, any>>('/setting/config/federationSettings');
        setSettings(normalizeFederationSettings(data));
      } catch {
        Toast.error(t('加载平台联邦设置失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const save = async () => {
    setSaving(true);
    try {
      const saved = await apiPost<Record<string, any>>('/setting/config/saveFederationSettings', settings);
      const next = normalizeFederationSettings(saved);
      setSettings(next);
      // 同步本地缓存，后台菜单门控刷新后即按新设置渲染
      cacheFederationSettings(next);
      Toast.success(t('保存成功，刷新页面后菜单按新设置显示'));
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin size="large" />
      </div>
    );
  }

  const setCapability = (key: keyof FederationSettingsValue['capabilities'], value: boolean) =>
    setSettings((current) => ({ ...current, capabilities: { ...current.capabilities, [key]: value } }));
  const setModule = (key: keyof FederationSettingsValue['modules'], value: boolean) =>
    setSettings((current) => ({ ...current, modules: { ...current.modules, [key]: value } }));

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16, maxWidth: 760 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Title heading={4} style={{ margin: 0 }}>
          {t('平台联邦')}
        </Title>
      </div>

      <Card title={t('总开关')}>
        <Space align="center" spacing={12}>
          <Switch
            checked={settings.enabled}
            onChange={(checked) => setSettings((current) => ({ ...current, enabled: checked }))}
          />
          <div style={{ display: 'grid', gap: 4 }}>
            <Text strong>{settings.enabled ? t('已开启') : t('已关闭（默认）')}</Text>
            <Text type="tertiary">
              {t('关闭时一级菜单「组织」整体不出现，/hub/* 接口后端返回 404；本设置页常驻不受影响。')}
            </Text>
          </div>
        </Space>
      </Card>

      <Card title={t('能力')}>
        <Space vertical align="start" spacing={10}>
          <Checkbox checked={settings.capabilities.supplyOut} onChange={(e) => setCapability('supplyOut', Boolean(e.target.checked))}>
            {t('对外供货（supplyOut）—— 本实例向外提供商品/数据')}
          </Checkbox>
          <Checkbox checked={settings.capabilities.supplyIn} onChange={(e) => setCapability('supplyIn', Boolean(e.target.checked))}>
            {t('接入供货（supplyIn）—— 本实例接入外部供货')}
          </Checkbox>
          <Checkbox checked={settings.capabilities.federation} onChange={(e) => setCapability('federation', Boolean(e.target.checked))}>
            {t('全联邦（federation）—— 实例间配对连接与互通')}
          </Checkbox>
        </Space>
      </Card>

      <Card title={t('模块')}>
        <Space vertical align="start" spacing={10}>
          <Checkbox checked={settings.modules.product} onChange={(e) => setModule('product', Boolean(e.target.checked))}>
            {t('商品模块')}
          </Checkbox>
          <Checkbox checked={settings.modules.member} onChange={(e) => setModule('member', Boolean(e.target.checked))}>
            {t('会员模块（P2 实现，本期仅开关）')}
          </Checkbox>
          <Checkbox checked={settings.modules.order} onChange={(e) => setModule('order', Boolean(e.target.checked))}>
            {t('订单模块')}
          </Checkbox>
        </Space>
      </Card>

      <Card title={t('本实例信息')}>
        <div style={{ display: 'grid', gap: 12 }}>
          <div>
            <Text strong>{t('实例 UUID（只读）')}</Text>
            <Input value={settings.instanceUuid} disabled placeholder={t('保存后由后端生成')} style={{ marginTop: 6 }} />
          </div>
          <div>
            <Text strong>{t('实例名称')}</Text>
            <Input
              value={settings.instanceName}
              onChange={(value) => setSettings((current) => ({ ...current, instanceName: value }))}
              placeholder={t('如：华东中枢')}
              style={{ marginTop: 6 }}
            />
          </div>
          <div>
            <Text strong>{t('对外地址')}</Text>
            <Input
              value={settings.instanceBaseUrl}
              onChange={(value) => setSettings((current) => ({ ...current, instanceBaseUrl: value }))}
              placeholder="https://example.com"
              style={{ marginTop: 6 }}
            />
          </div>
        </div>
      </Card>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>
    </div>
  );
}
