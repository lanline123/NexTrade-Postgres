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
 * 买家取消退款（设置 → 商城设置 → 买家取消退款）。我方新增，
 * 论坛帖 347748，用户 2026-11-21 拍板口径。
 *
 * 已支付未发货的买家可在订单列表/详情自助「取消并退款」：
 * 生成来源为「买家取消」的退款申请，走既有财务退款链出款。
 * 配置存后端 config code `paidCancelRefund`：
 * - 主开关默认关：关时买家侧不出现入口，全站零行为变化；
 * - 小额免审（默认关）+ 免审金额上限：开启且退款额 ≤ 上限时
 *   申请自动通过、直达出款，记录上标明系统免审；
 * - 取消后进财务审核队列（默认开）：未命中免审时申请进财务
 *   审核队列，审核通过才出款；关闭时未命中免审也直接出款。
 */

interface PaidCancelRefundConfig {
  enabled: boolean;
  smallExemptEnabled: boolean;
  smallExemptLimit: string;
  financeAuditEnabled: boolean;
}

const DEFAULT_CONFIG: PaidCancelRefundConfig = {
  enabled: false,
  smallExemptEnabled: false,
  smallExemptLimit: '0',
  financeAuditEnabled: true,
};

export default function PaidCancelRefundSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [cfg, setCfg] = useState<PaidCancelRefundConfig>(DEFAULT_CONFIG);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<PaidCancelRefundConfig>(
          '/setting/config/paidCancelRefundSettings',
        );
        if (data && typeof data === 'object') {
          setCfg({
            enabled: data.enabled === true,
            smallExemptEnabled: data.smallExemptEnabled === true,
            smallExemptLimit:
              data.smallExemptLimit != null ? String(data.smallExemptLimit) : '0',
            financeAuditEnabled: data.financeAuditEnabled !== false,
          });
        }
      } catch {
        Toast.error(t('加载买家取消退款设置失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/savePaidCancelRefund', {
        enabled: cfg.enabled,
        smallExemptEnabled: cfg.smallExemptEnabled,
        smallExemptLimit: cfg.smallExemptLimit || '0',
        financeAuditEnabled: cfg.financeAuditEnabled,
      });
      Toast.success(t('保存成功'));
    } catch {
      Toast.error(t('保存失败'));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 24, textAlign: 'center' }}>
        <Spin size="large" />
      </div>
    );
  }

  const limitDisabled = !cfg.enabled || !cfg.smallExemptEnabled;

  return (
    <div style={{ padding: 24, maxWidth: 860 }}>
      <Title heading={4} style={{ marginBottom: 8 }}>
        {t('买家取消退款')}
      </Title>
      <Text type="tertiary">
        {t(
          '已支付未发货的买家可自助取消订单并退款，退款走财务退款链出款（来源标记为「买家取消」）。已发货订单仍走售后通道。',
        )}
      </Text>

      <Card style={{ marginTop: 16 }}>
        <Space vertical align="start" spacing={20} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Switch
              checked={cfg.enabled}
              onChange={(v) => setCfg((p) => ({ ...p, enabled: v }))}
            />
            <div>
              <Text strong>{t('已付未发货买家可取消退款')}</Text>
              <br />
              <Text type="tertiary" size="small">
                {t('默认关闭。开启后买家订单列表/详情出现「取消并退款」入口；关闭时全站无此入口。')}
              </Text>
            </div>
          </Space>

          <Space align="center" spacing={12}>
            <Switch
              checked={cfg.smallExemptEnabled}
              disabled={!cfg.enabled}
              onChange={(v) => setCfg((p) => ({ ...p, smallExemptEnabled: v }))}
            />
            <div>
              <Text strong>{t('小额免审')}</Text>
              <br />
              <Text type="tertiary" size="small">
                {t('开启后，退款金额不超过免审上限的申请自动通过、直接出款，退款记录标明系统免审。')}
              </Text>
            </div>
          </Space>

          <Space align="center" spacing={12}>
            <Text style={{ width: 42 }} />
            <div>
              <Text>{t('免审金额上限（元）')}</Text>
              <br />
              <InputNumber
                value={Number(cfg.smallExemptLimit) || 0}
                min={0}
                precision={2}
                disabled={limitDisabled}
                style={{ width: 220, marginTop: 6 }}
                onChange={(v) =>
                  setCfg((p) => ({
                    ...p,
                    smallExemptLimit: v == null ? '0' : String(v),
                  }))
                }
              />
              <br />
              <Text type="tertiary" size="small">
                {t('填 0 表示不设免审（即使开启小额免审也不会命中）。')}
              </Text>
            </div>
          </Space>

          <Space align="center" spacing={12}>
            <Switch
              checked={cfg.financeAuditEnabled}
              disabled={!cfg.enabled}
              onChange={(v) => setCfg((p) => ({ ...p, financeAuditEnabled: v }))}
            />
            <div>
              <Text strong>{t('取消后进财务审核队列')}</Text>
              <br />
              <Text type="tertiary" size="small">
                {t('默认开启。未命中小额免审的申请进入财务审核队列，审核通过后才出款；关闭后未命中免审的申请也直接出款。')}
              </Text>
            </div>
          </Space>
        </Space>
      </Card>

      <div style={{ marginTop: 20 }}>
        <Button type="primary" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>
    </div>
  );
}
