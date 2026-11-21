import { useEffect, useState } from 'react';
import { Button, Card, InputNumber, Select, Space, Spin, Switch, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 拍卖设置（营销 → 营销管理 → 拍卖设置）。
 * 全部策略项存后端 config code `auctionSettings`（整块 JSON），
 * 未保存过时后端返回出厂推荐值。金额/比例字段后端以字符串返回，保存时同样回传字符串。
 */

interface BidStep {
  maxPrice: string | null;
  step: string;
}

interface AuctionSettingsConfig {
  requireCertification: boolean;
  depositRatePercent: string;
  depositMin: string;
  depositMax: string;
  breachDisposal: string;
  finalPayHours: number;
  penaltyRatePercent: string;
  penaltyCapRatePercent: string;
  secondBreachBanDays: number;
  permanentBreachCount: number;
  merchantAuctionEnabled: boolean;
  reservePriceEnabled: boolean;
  buyoutEnabled: boolean;
  noReasonReturnAllowed: boolean;
  extendWindowSeconds: number;
  extendBySeconds: number;
  extendMaxSeconds: number;
  bidSteps: BidStep[];
  returnPolicyText?: string;
}

const DEFAULTS: AuctionSettingsConfig = {
  requireCertification: true,
  depositRatePercent: '10',
  depositMin: '100',
  depositMax: '50000',
  breachDisposal: 'reauction',
  finalPayHours: 48,
  penaltyRatePercent: '50',
  penaltyCapRatePercent: '20',
  secondBreachBanDays: 30,
  permanentBreachCount: 3,
  merchantAuctionEnabled: false,
  reservePriceEnabled: false,
  buyoutEnabled: true,
  noReasonReturnAllowed: false,
  extendWindowSeconds: 300,
  extendBySeconds: 300,
  extendMaxSeconds: 86400,
  bidSteps: [
    { maxPrice: '1000', step: '10' },
    { maxPrice: '10000', step: '50' },
    { maxPrice: '50000', step: '100' },
    { maxPrice: '200000', step: '500' },
    { maxPrice: '1000000', step: '1000' },
    { maxPrice: null, step: '5000' },
  ],
};

function Row({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', alignItems: 'flex-start', padding: '10px 0', borderBottom: '1px solid var(--semi-color-border)' }}>
      <div style={{ width: 240, flexShrink: 0 }}>
        <Text strong>{label}</Text>
        {hint ? (
          <div>
            <Text type="tertiary" size="small">
              {hint}
            </Text>
          </div>
        ) : null}
      </div>
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  );
}

export default function AuctionSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [cfg, setCfg] = useState<AuctionSettingsConfig>(DEFAULTS);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<AuctionSettingsConfig>('/promotion/auction/settings');
        if (data && typeof data === 'object') setCfg({ ...DEFAULTS, ...data });
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const set = <K extends keyof AuctionSettingsConfig>(key: K, value: AuctionSettingsConfig[K]) =>
    setCfg((prev) => ({ ...prev, [key]: value }));

  const save = async () => {
    setSaving(true);
    try {
      const saved = await apiPost<AuctionSettingsConfig>('/promotion/auction/settings/save', cfg);
      if (saved && typeof saved === 'object') setCfg({ ...DEFAULTS, ...saved });
      Toast.success({ content: t('保存成功') });
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

  const money = (value: string, onChange: (v: string) => void, width = 160) => (
    <InputNumber
      style={{ width }}
      min={0}
      value={value === '' ? undefined : Number(value)}
      onChange={(v) => onChange(v == null ? '' : String(v))}
    />
  );

  return (
    <div style={{ padding: 16 }}>
      <Title heading={4} style={{ marginBottom: 12 }}>
        {t('拍卖设置')}
      </Title>

      <Card title={t('参拍与保证金')} style={{ marginBottom: 16 }}>
        <Row label={t('参拍门槛')} hint={t('开启后仅企业认证通过的会员可冻结保证金与出价')}>
          <Switch checked={cfg.requireCertification} onChange={(v) => set('requireCertification', v)} />
          <Text style={{ marginLeft: 8 }}>{cfg.requireCertification ? t('须企业认证') : t('登录即可参拍')}</Text>
        </Row>
        <Row label={t('保证金比例')} hint={t('按起拍价计算，冻结于会员余额')}>
          {money(cfg.depositRatePercent, (v) => set('depositRatePercent', v), 120)}
          <Text style={{ marginLeft: 6 }}>%</Text>
        </Row>
        <Row label={t('保证金保底 / 封顶')}>
          <Space>
            {money(cfg.depositMin, (v) => set('depositMin', v))}
            <Text>~</Text>
            {money(cfg.depositMax, (v) => set('depositMax', v))}
            <Text type="tertiary">{t('元')}</Text>
          </Space>
        </Row>
        <Row label={t('商户上拍（二期预留）')} hint={t('一期仅平台自营，此开关暂不生效；二期开放商户自助上拍时在商户入口接线校验')}>
          <Switch checked={cfg.merchantAuctionEnabled} onChange={(v) => set('merchantAuctionEnabled', v)} />
          <Text type="tertiary" style={{ marginLeft: 8 }}>{t('二期预留')}</Text>
        </Row>
      </Card>

      <Card title={t('竞价规则')} style={{ marginBottom: 16 }}>
        <Row label={t('保留价')} hint={t('开启后建拍品可设置保留价，未达保留价流拍')}>
          <Switch checked={cfg.reservePriceEnabled} onChange={(v) => set('reservePriceEnabled', v)} />
        </Row>
        <Row label={t('一口价')} hint={t('开启后卖家可为拍品设置一口价，买家可直接拍下成交')}>
          <Switch checked={cfg.buyoutEnabled} onChange={(v) => set('buyoutEnabled', v)} />
        </Row>
        <Row label={t('延时防狙击')} hint={t('结束前窗口内出价自动顺延；累计顺延不超过上限（秒）')}>
          <Space>
            <Text type="tertiary">{t('窗口')}</Text>
            <InputNumber style={{ width: 110 }} min={0} value={cfg.extendWindowSeconds} onChange={(v) => set('extendWindowSeconds', Number(v ?? 0))} />
            <Text type="tertiary">{t('顺延')}</Text>
            <InputNumber style={{ width: 110 }} min={0} value={cfg.extendBySeconds} onChange={(v) => set('extendBySeconds', Number(v ?? 0))} />
            <Text type="tertiary">{t('上限')}</Text>
            <InputNumber style={{ width: 120 }} min={0} value={cfg.extendMaxSeconds} onChange={(v) => set('extendMaxSeconds', Number(v ?? 0))} />
          </Space>
        </Row>
        <Row label={t('加价阶梯')} hint={t('按当前价所在区间取最小加价幅度；最后一档不设价格上限')}>
          <div>
            {cfg.bidSteps.map((row, idx) => (
              <Space key={idx} style={{ marginBottom: 6 }}>
                <Text type="tertiary">{t('当前价 ≤')}</Text>
                {row.maxPrice === null ? (
                  <Text style={{ width: 130, display: 'inline-block' }}>{t('不限')}</Text>
                ) : (
                  <InputNumber
                    style={{ width: 130 }}
                    min={0}
                    value={Number(row.maxPrice)}
                    onChange={(v) => {
                      const next = [...cfg.bidSteps];
                      next[idx] = { ...row, maxPrice: v == null ? null : String(v) };
                      set('bidSteps', next);
                    }}
                  />
                )}
                <Text type="tertiary">{t('加价')}</Text>
                <InputNumber
                  style={{ width: 110 }}
                  min={0}
                  value={Number(row.step)}
                  onChange={(v) => {
                    const next = [...cfg.bidSteps];
                    next[idx] = { ...row, step: String(v ?? 0) };
                    set('bidSteps', next);
                  }}
                />
                <Button
                  size="small"
                  type="danger"
                  theme="borderless"
                  onClick={() => set('bidSteps', cfg.bidSteps.filter((_, i) => i !== idx))}
                >
                  {t('删除')}
                </Button>
              </Space>
            ))}
            <div>
              <Button
                size="small"
                onClick={() => set('bidSteps', [...cfg.bidSteps, { maxPrice: '10000', step: '50' }])}
              >
                {t('添加一档')}
              </Button>
            </div>
          </div>
        </Row>
      </Card>

      <Card title={t('成交与违约')} style={{ marginBottom: 16 }}>
        <Row label={t('尾款支付期限')}>
          <InputNumber style={{ width: 120 }} min={1} value={cfg.finalPayHours} onChange={(v) => set('finalPayHours', Number(v ?? 48))} />
          <Text style={{ marginLeft: 6 }}>{t('小时')}</Text>
        </Row>
        <Row label={t('违约扣罚')} hint={t('扣保证金的比例，且不超过中标价的上限比例')}>
          <Space>
            {money(cfg.penaltyRatePercent, (v) => set('penaltyRatePercent', v), 110)}
            <Text>%</Text>
            <Text type="tertiary">{t('上限')}</Text>
            {money(cfg.penaltyCapRatePercent, (v) => set('penaltyCapRatePercent', v), 110)}
            <Text>%</Text>
          </Space>
        </Row>
        <Row label={t('违约封禁')} hint={t('近 12 个月内累计违约：第 2 次起临时封禁，达到永久次数永久取消参拍资格')}>
          <Space>
            <Text type="tertiary">{t('第 2 次封')}</Text>
            <InputNumber style={{ width: 100 }} min={0} value={cfg.secondBreachBanDays} onChange={(v) => set('secondBreachBanDays', Number(v ?? 0))} />
            <Text type="tertiary">{t('天；第')}</Text>
            <InputNumber style={{ width: 80 }} min={1} value={cfg.permanentBreachCount} onChange={(v) => set('permanentBreachCount', Number(v ?? 3))} />
            <Text type="tertiary">{t('次永久封禁')}</Text>
          </Space>
        </Row>
        <Row label={t('违约后商品处置')} hint={t('一期仅支持重新上拍；顺延次高价为二期能力，暂不可选（后端收到其他值也会归一为重新上拍）')}>
          <Select
            style={{ width: 200 }}
            value={cfg.breachDisposal}
            onChange={(v) => set('breachDisposal', String(v))}
            optionList={[
              { label: t('重新上拍（回草稿待重新排期）'), value: 'reauction' },
              { label: t('顺延次高价（二期）'), value: 'roll_down', disabled: true },
            ]}
          />
        </Row>
        <Row label={t('退换货口径')} hint={cfg.returnPolicyText || ''}>
          <Switch checked={cfg.noReasonReturnAllowed} onChange={(v) => set('noReasonReturnAllowed', v)} />
          <Text style={{ marginLeft: 8 }}>
            {cfg.noReasonReturnAllowed ? t('与普通商品同口径') : t('仅质量问题可退')}
          </Text>
        </Row>
      </Card>

      <Button theme="solid" type="primary" loading={saving} onClick={save}>
        {t('保存设置')}
      </Button>
    </div>
  );
}
