import { useEffect, useState } from 'react';
import { Button, Card, Radio, Space, Spin, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import { cacheSystemMode, normalizeSystemMode, type SystemMode } from '../systemMode';

const { Text, Title } = Typography;

/**
 * 系统模式设置（设置 → 商城设置 → 系统模式）—— 我方新增，原版无此设置。
 *
 * 照 BuyStyleSettings 形态：配置存后端 config code `systemMode`，
 * 后台读 GET /setting/config/systemModeSettings、写 POST /setting/config/saveSystemMode，
 * 前台通过 GET /api/common/config/systemMode 读取。默认 all（全功能），
 * 门控只做入口/菜单渲染层隐藏，不删数据、不拦直达链接（见 system-mode-design.md）。
 */

const MODE_OPTIONS: Array<{ value: SystemMode; label: string; desc: string }> = [
  {
    value: 'all',
    label: '全功能',
    desc: '商城与礼品卡功能全部开启（默认）。存量部署升级后行为与现状完全一致。',
  },
  {
    value: 'giftcard',
    label: '礼品卡系统',
    desc: '以购卡、兑换、我的礼品卡与福利商品为一级入口；秒杀、拼团、优惠券等普通商城营销入口收起。已发出的礼品卡仍可兑换、抵扣。',
  },
  {
    value: 'mall',
    label: '商城系统',
    desc: '普通商城形态：礼品卡整体收起（无购卡、无兑换入口、无结算抵扣区，后台隐藏礼品卡菜单）。已有订单与已发卡的直达处理不受影响。',
  },
];

export default function SystemModeSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [mode, setMode] = useState<SystemMode>('all');

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<Record<string, any>>('/setting/config/systemModeSettings');
        setMode(normalizeSystemMode(data));
      } catch {
        Toast.error(t('加载系统模式设置失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveSystemMode', { mode });
      // 同步本地缓存，后台菜单门控刷新后即按新模式渲染
      cacheSystemMode(mode);
      Toast.success(t('保存成功，刷新页面后菜单按新模式显示'));
    } catch {
      Toast.error(t('保存失败'));
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

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Title heading={4} style={{ margin: 0 }}>
          {t('系统模式')}
        </Title>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>

      <Space vertical align="start" spacing={12} style={{ width: '100%' }}>
        {MODE_OPTIONS.map((opt) => {
          const selected = mode === opt.value;
          return (
            <div key={opt.value} style={{ width: '100%', maxWidth: 720 }} onClick={() => setMode(opt.value)}>
              <Card
                style={{
                  width: '100%',
                  cursor: 'pointer',
                  borderColor: selected ? 'var(--semi-color-primary)' : undefined,
                  background: selected ? 'var(--semi-color-primary-light-default)' : undefined,
                }}
              >
                <Space align="start" spacing={10}>
                  <Radio checked={selected} value={opt.value} onChange={() => setMode(opt.value)} />
                  <div style={{ display: 'grid', gap: 4 }}>
                    <Text strong>
                      {t(opt.label)}
                      {opt.value === 'all' ? <Text type="tertiary">（{t('默认')}）</Text> : null}
                    </Text>
                    <Text type="tertiary">{t(opt.desc)}</Text>
                  </div>
                </Space>
              </Card>
            </div>
          );
        })}
      </Space>

      <Text type="tertiary">
        {t('说明：切换模式只隐藏对应入口与菜单，不删除、不修改任何业务数据；切回「全功能」即原样恢复。已发出的礼品卡在任何模式下仍可兑换与结算抵扣。')}
      </Text>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>
    </div>
  );
}
