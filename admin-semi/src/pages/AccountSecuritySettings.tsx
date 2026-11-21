import { useEffect, useState } from 'react';
import {
  Button,
  Card,
  InputNumber,
  Space,
  Spin,
  Switch,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 账号安全（设置 → 商城设置 → 账号安全）。我方新增（论坛 B 类第 6 项，
 * 用户 2026-11-21 拍板）。
 *
 * 密码登录失败计数与临时锁定：按账号计数（会员端 + 后台端，作用域
 * 分开），在「锁定时长」窗口内失败达到「最大失败次数」即锁定账号
 * 对应时长；锁定期间密码登录直接拒绝并提示剩余时间，登录成功清零
 * 计数，锁定到期自动解除。短信/验证码登录不计入、也不受锁定影响
 * （手机号本人仍可验证码登录）。配置存后端 config code
 * `accountSecurity`，登录链实时读取生效。
 */

interface AccountSecurityConfig {
  enabled: boolean;
  maxFailures: number;
  lockMinutes: number;
}

const DEFAULT_CFG: AccountSecurityConfig = {
  enabled: true,
  maxFailures: 5,
  lockMinutes: 15,
};

export default function AccountSecuritySettings() {
  const { t } = useI18n();
  const [cfg, setCfg] = useState<AccountSecurityConfig>(DEFAULT_CFG);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let alive = true;
    apiGet<AccountSecurityConfig>('/setting/config/accountSecuritySettings')
      .then((res) => {
        if (alive && res) setCfg({ ...DEFAULT_CFG, ...res });
      })
      .catch(() => {
        if (alive) Toast.error(t('加载失败，请稍后重试'));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [t]);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveAccountSecurity', cfg);
      Toast.success(t('保存成功'));
    } catch {
      Toast.error(t('保存失败，请稍后重试'));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Spin spinning={loading}>
      <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <Title heading={4} style={{ margin: 0 }}>
            {t('账号安全')}
          </Title>
          <Button theme="solid" type="primary" loading={saving} onClick={save}>
            {t('保存')}
          </Button>
        </div>

        <Card>
          <Space vertical align="start" spacing={14} style={{ width: '100%' }}>
            <Space align="center" spacing={12}>
              <Switch
                checked={cfg.enabled}
                onChange={(v) => setCfg((p) => ({ ...p, enabled: v }))}
              />
              <Text strong>{t('启用登录失败锁定')}</Text>
              <Text type="tertiary">
                {t('关闭后不再统计失败次数，已锁定的账号在到期前仍保持锁定（可手动解锁）')}
              </Text>
            </Space>
            <Space align="center" spacing={12}>
              <Text>{t('最大失败次数')}</Text>
              <InputNumber
                min={1}
                max={20}
                precision={0}
                style={{ width: 100 }}
                value={cfg.maxFailures}
                onChange={(v) => setCfg((p) => ({ ...p, maxFailures: Number(v ?? 5) }))}
              />
              <Text type="tertiary">{t('在锁定时长窗口内，密码登录连续失败达到该次数即锁定')}</Text>
            </Space>
            <Space align="center" spacing={12}>
              <Text>{t('锁定时长（分钟）')}</Text>
              <InputNumber
                min={1}
                max={1440}
                precision={0}
                style={{ width: 100 }}
                value={cfg.lockMinutes}
                onChange={(v) => setCfg((p) => ({ ...p, lockMinutes: Number(v ?? 15) }))}
              />
              <Text type="tertiary">{t('同时作为失败计数窗口；到期后自动解除锁定')}</Text>
            </Space>
            <Text type="tertiary">
              {t(
                '说明：锁定按账号维度生效（会员端与后台端分别计数），不封禁 IP，避免共享网络出口误伤；短信/验证码登录不计入失败次数，锁定期间仍可使用；登录成功会清零失败计数。会员被锁定后可在「会员详情 → 登录安全」中查看状态并手动解除。',
              )}
            </Text>
          </Space>
        </Card>
      </div>
    </Spin>
  );
}
