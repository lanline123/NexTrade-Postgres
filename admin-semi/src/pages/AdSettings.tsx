/**
 * 平台「投流定价」设置（设置 > 商城设置，与价格显示设置同组同形态）。
 *
 * 配置存后端 config code `adSettings`（整块 JSON）：
 * - GET  /adminapi/setting/config/adSettings      读取生效配置
 * - POST /adminapi/setting/config/saveAdSettings  整块覆盖保存（后端清洗钳制）
 * 前台经 GET /api/common/config/adSettings 消费同一份配置。
 */
import { useEffect, useState } from 'react';
import { Button, Card, InputNumber, Space, Spin, Switch, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import { AD_POSITIONS, type AdSettings } from './adCampaignShared';

const { Text, Title } = Typography;

const DEFAULTS: AdSettings = {
  prices: { shop_street: 30, product_search: 20, product_list: 15, recommend_feed: 20 },
  minDays: 3,
  pinLimit: 3,
  promotedPerPage: 2,
  recommendEnabled: true,
  needAudit: true,
};

export default function AdSettingsPage() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [cfg, setCfg] = useState<AdSettings>(DEFAULTS);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<AdSettings>('/setting/config/adSettings');
        if (data && typeof data === 'object') {
          setCfg({
            prices: { ...DEFAULTS.prices, ...(data.prices || {}) },
            minDays: Number.isFinite(Number(data.minDays)) ? Number(data.minDays) : DEFAULTS.minDays,
            pinLimit: Number.isFinite(Number(data.pinLimit)) ? Number(data.pinLimit) : DEFAULTS.pinLimit,
            promotedPerPage: Number.isFinite(Number(data.promotedPerPage)) ? Number(data.promotedPerPage) : DEFAULTS.promotedPerPage,
            recommendEnabled: data.recommendEnabled !== false,
            needAudit: data.needAudit !== false,
          });
        }
      } catch {
        Toast.error(t('加载投流设置失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveAdSettings', cfg);
      Toast.success(t('保存成功'));
    } catch (e: any) {
      Toast.error({ content: e.message || t('保存失败') });
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
          {t('投流定价')}
        </Title>
        <Button type="primary" theme="solid" loading={saving} onClick={() => void save()}>
          {t('保存')}
        </Button>
      </div>

      <Card title={t('各投放位单价（元/天）')}>
        <Space vertical align="start" spacing={12}>
          {AD_POSITIONS.map((p) => (
            <Space key={p.value} align="center" spacing={12}>
              <Text style={{ width: 130 }}>{t(p.label)}</Text>
              <InputNumber
                min={0}
                max={100000}
                precision={2}
                style={{ width: 160 }}
                value={cfg.prices[p.value]}
                onChange={(v) => setCfg((prev) => ({ ...prev, prices: { ...prev.prices, [p.value]: Number(v ?? 0) } }))}
              />
              <Text type="tertiary">{t('元/天（商家下单时快照，改价不影响已购投放）')}</Text>
            </Space>
          ))}
        </Space>
      </Card>

      <Card title={t('投放规则')}>
        <Space vertical align="start" spacing={12}>
          <Space align="center" spacing={12}>
            <Text style={{ width: 130 }}>{t('最短购买天数')}</Text>
            <InputNumber
              min={1}
              max={30}
              precision={0}
              style={{ width: 160 }}
              value={cfg.minDays}
              onChange={(v) => setCfg((prev) => ({ ...prev, minDays: Number(v ?? 1) }))}
            />
            <Text type="tertiary">{t('天')}</Text>
          </Space>
          <Space align="center" spacing={12}>
            <Text style={{ width: 130 }}>{t('置顶位上限')}</Text>
            <InputNumber
              min={0}
              max={10}
              precision={0}
              style={{ width: 160 }}
              value={cfg.pinLimit}
              onChange={(v) => setCfg((prev) => ({ ...prev, pinLimit: Number(v ?? 0) }))}
            />
            <Text type="tertiary">{t('店铺街最多展示的平台置顶店铺数，0 = 关闭置顶')}</Text>
          </Space>
          <Space align="center" spacing={12}>
            <Text style={{ width: 130 }}>{t('每页推广上限')}</Text>
            <InputNumber
              min={0}
              max={10}
              precision={0}
              style={{ width: 160 }}
              value={cfg.promotedPerPage}
              onChange={(v) => setCfg((prev) => ({ ...prev, promotedPerPage: Number(v ?? 0) }))}
            />
            <Text type="tertiary">{t('店铺街每页最多插入的付费推广店铺数，溢出顺延到后续页')}</Text>
          </Space>
        </Space>
      </Card>

      <Card title={t('开关')}>
        <Space vertical align="start" spacing={12}>
          <Space align="center" spacing={12}>
            <Switch checked={cfg.recommendEnabled} onChange={(v) => setCfg((prev) => ({ ...prev, recommendEnabled: v }))} />
            <Text strong>{t('智能推荐（千人千面）')}</Text>
            <Text type="tertiary">{t('关闭后店铺街推荐排序只走置顶/推广 + 店铺质量兜底，不再使用个人行为信号')}</Text>
          </Space>
          <Space align="center" spacing={12}>
            <Switch checked={cfg.needAudit} onChange={(v) => setCfg((prev) => ({ ...prev, needAudit: v }))} />
            <Text strong>{t('投放需要审核')}</Text>
            <Text type="tertiary">{t('开启时商家投放付款后需平台审核通过才生效；关闭则付款后直接排期')}</Text>
          </Space>
        </Space>
      </Card>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button type="primary" theme="solid" loading={saving} onClick={() => void save()}>
          {t('保存')}
        </Button>
      </div>
    </div>
  );
}
