import { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Card,
  Input,
  InputNumber,
  Select,
  Space,
  Spin,
  Switch,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { IconDelete, IconPlus } from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 全站价格显示设置（设置 → 商城设置 → 价格显示设置）。
 *
 * 大额商品展示价按币种单位阶梯缩写：人民币 万/亿、美元 K/M/B、印度卢比 K/L/Cr 等。
 * 每条规则 = 门槛（金额 ≥ at 时启用）+ 除数（显示值 = 金额 / div）+ 后缀（自由文本，
 * 如 万、百万、K、M）。配置存后端 config code `priceDisplay`，前台通过
 * GET /api/common/config/priceDisplay 读取。购物车/结算/订单等对账金额不受影响，
 * 始终精确显示。
 */

interface PriceUnitRule {
  at: number;
  div: number;
  suffix: string;
}

interface PriceDisplayConfig {
  enabled: boolean;
  decimals: number;
  trimZeros: boolean;
  currencies: Record<string, PriceUnitRule[]>;
}

interface CurrencyMeta {
  code: string;
  name: string;
  symbol: string;
}

/** 主要货币清单（添加币种时选）。 */
const MASTER_CURRENCIES: CurrencyMeta[] = [
  { code: 'CNY', name: '人民币', symbol: '¥' },
  { code: 'USD', name: '美元', symbol: '$' },
  { code: 'EUR', name: '欧元', symbol: '€' },
  { code: 'GBP', name: '英镑', symbol: '£' },
  { code: 'JPY', name: '日元', symbol: '¥' },
  { code: 'KRW', name: '韩元', symbol: '₩' },
  { code: 'HKD', name: '港币', symbol: 'HK$' },
  { code: 'TWD', name: '新台币', symbol: 'NT$' },
  { code: 'AUD', name: '澳大利亚元', symbol: 'A$' },
  { code: 'CAD', name: '加拿大元', symbol: 'C$' },
  { code: 'SGD', name: '新加坡元', symbol: 'S$' },
  { code: 'CHF', name: '瑞士法郎', symbol: 'CHF' },
  { code: 'NZD', name: '新西兰元', symbol: 'NZ$' },
  { code: 'INR', name: '印度卢比', symbol: '₹' },
  { code: 'RUB', name: '俄罗斯卢布', symbol: '₽' },
  { code: 'BRL', name: '巴西雷亚尔', symbol: 'R$' },
  { code: 'MYR', name: '马来西亚林吉特', symbol: 'RM' },
  { code: 'PHP', name: '菲律宾比索', symbol: '₱' },
  { code: 'PLN', name: '波兰兹罗提', symbol: 'zł' },
  { code: 'THB', name: '泰铢', symbol: '฿' },
  { code: 'VND', name: '越南盾', symbol: '₫' },
  { code: 'IDR', name: '印尼盾', symbol: 'Rp' },
  { code: 'TRY', name: '土耳其里拉', symbol: '₺' },
  { code: 'ZAR', name: '南非兰特', symbol: 'R' },
  { code: 'MXN', name: '墨西哥比索', symbol: 'Mex$' },
  { code: 'SAR', name: '沙特里亚尔', symbol: 'SR' },
  { code: 'AED', name: '阿联酋迪拉姆', symbol: 'DH' },
  { code: 'QAR', name: '卡塔尔里亚尔', symbol: 'QR' },
  { code: 'KWD', name: '科威特第纳尔', symbol: 'KD' },
  { code: 'SEK', name: '瑞典克朗', symbol: 'kr' },
  { code: 'NOK', name: '挪威克朗', symbol: 'kr' },
  { code: 'DKK', name: '丹麦克朗', symbol: 'kr' },
  { code: 'CZK', name: '捷克克朗', symbol: 'Kč' },
  { code: 'HUF', name: '匈牙利福林', symbol: 'Ft' },
  { code: 'ILS', name: '以色列谢克尔', symbol: '₪' },
];

const WESTERN: PriceUnitRule[] = [
  { at: 10000, div: 1000, suffix: 'K' },
  { at: 1000000, div: 1000000, suffix: 'M' },
  { at: 1000000000, div: 1000000000, suffix: 'B' },
];

/** 新增币种时的默认阶梯（与前台内置默认一致）。 */
const DEFAULT_LADDERS: Record<string, PriceUnitRule[]> = {
  CNY: [
    { at: 10000, div: 10000, suffix: '万' },
    { at: 100000000, div: 100000000, suffix: '亿' },
  ],
  HKD: [
    { at: 10000, div: 10000, suffix: '万' },
    { at: 100000000, div: 100000000, suffix: '億' },
  ],
  TWD: [
    { at: 10000, div: 10000, suffix: '萬' },
    { at: 100000000, div: 100000000, suffix: '億' },
  ],
  JPY: [
    { at: 10000, div: 10000, suffix: '万' },
    { at: 100000000, div: 100000000, suffix: '億' },
  ],
  KRW: [
    { at: 10000, div: 10000, suffix: '만' },
    { at: 100000000, div: 100000000, suffix: '억' },
  ],
  INR: [
    { at: 10000, div: 1000, suffix: 'K' },
    { at: 100000, div: 100000, suffix: 'L' },
    { at: 10000000, div: 10000000, suffix: 'Cr' },
  ],
  RUB: [
    { at: 10000, div: 1000, suffix: ' тыс.' },
    { at: 1000000, div: 1000000, suffix: ' млн' },
    { at: 1000000000, div: 1000000000, suffix: ' млрд' },
  ],
  IDR: [
    { at: 10000, div: 1000, suffix: 'K' },
    { at: 1000000, div: 1000000, suffix: 'Jt' },
    { at: 1000000000, div: 1000000000, suffix: 'M' },
  ],
  VND: [
    { at: 100000, div: 1000, suffix: 'K' },
    { at: 1000000, div: 1000000, suffix: 'Tr' },
    { at: 1000000000, div: 1000000000, suffix: 'Tỷ' },
  ],
};

function metaOf(code: string): CurrencyMeta {
  return MASTER_CURRENCIES.find((c) => c.code === code) || { code, name: code, symbol: '' };
}

/** 与前台一致的预览格式化（amount 为该币种金额，未换算）。 */
function previewFormat(cfg: PriceDisplayConfig, code: string, amount: number): string {
  const symbol = metaOf(code).symbol || '¤';
  if (!cfg.enabled) return `${symbol}${amount.toFixed(2)}`;
  const rules = cfg.currencies[code];
  const ladder = rules === undefined ? WESTERN : rules;
  let rule: PriceUnitRule | null = null;
  for (const r of ladder) {
    if (amount >= r.at && (!rule || r.at > rule.at)) rule = r;
  }
  if (!rule) return `${symbol}${amount.toFixed(2)}`;
  let text = (amount / rule.div).toFixed(cfg.decimals);
  if (cfg.trimZeros && text.includes('.')) text = text.replace(/\.?0+$/, '');
  return `${symbol}${text}${rule.suffix}`;
}

const SAMPLE_AMOUNTS = [8888, 1000000, 123456789];

export default function PriceDisplaySettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [cfg, setCfg] = useState<PriceDisplayConfig>({
    enabled: true,
    decimals: 2,
    trimZeros: true,
    currencies: {},
  });
  const [newCurrency, setNewCurrency] = useState<string>('');

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<PriceDisplayConfig>('/setting/config/priceDisplaySettings');
        if (data && typeof data === 'object') {
          setCfg({
            enabled: data.enabled !== false,
            decimals: Number.isFinite(Number(data.decimals)) ? Number(data.decimals) : 2,
            trimZeros: data.trimZeros !== false,
            currencies: data.currencies && typeof data.currencies === 'object' ? data.currencies : {},
          });
        }
      } catch {
        Toast.error(t('加载价格显示设置失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const currencyCodes = useMemo(
    () => Object.keys(cfg.currencies).sort((a, b) => a.localeCompare(b)),
    [cfg.currencies],
  );
  const addableCurrencies = useMemo(
    () => MASTER_CURRENCIES.filter((c) => !(c.code in cfg.currencies)),
    [cfg.currencies],
  );

  const updateRule = (code: string, index: number, patch: Partial<PriceUnitRule>) => {
    setCfg((prev) => ({
      ...prev,
      currencies: {
        ...prev.currencies,
        [code]: prev.currencies[code].map((r, i) => (i === index ? { ...r, ...patch } : r)),
      },
    }));
  };

  const removeRule = (code: string, index: number) => {
    setCfg((prev) => ({
      ...prev,
      currencies: {
        ...prev.currencies,
        [code]: prev.currencies[code].filter((_, i) => i !== index),
      },
    }));
  };

  const addRule = (code: string) => {
    setCfg((prev) => {
      const rules = prev.currencies[code] || [];
      const lastAt = rules.length ? Math.max(...rules.map((r) => r.at)) : 0;
      const at = lastAt > 0 ? lastAt * 100 : 10000;
      return {
        ...prev,
        currencies: {
          ...prev.currencies,
          [code]: [...rules, { at, div: at, suffix: '' }],
        },
      };
    });
  };

  const removeCurrency = (code: string) => {
    setCfg((prev) => {
      const next = { ...prev.currencies };
      delete next[code];
      return { ...prev, currencies: next };
    });
  };

  const addCurrency = () => {
    if (!newCurrency) return;
    const ladder = (DEFAULT_LADDERS[newCurrency] || WESTERN).map((r) => ({ ...r }));
    setCfg((prev) => ({ ...prev, currencies: { ...prev.currencies, [newCurrency]: ladder } }));
    setNewCurrency('');
  };

  const save = async () => {
    // 保存前按门槛升序整理每条阶梯，和前台消费口径一致
    const currencies: Record<string, PriceUnitRule[]> = {};
    for (const [code, rules] of Object.entries(cfg.currencies)) {
      currencies[code] = [...rules]
        .filter((r) => Number(r.at) > 0 && Number(r.div) > 0)
        .sort((a, b) => a.at - b.at);
    }
    setSaving(true);
    try {
      await apiPost('/setting/config/savePriceDisplay', {
        enabled: cfg.enabled,
        decimals: cfg.decimals,
        trimZeros: cfg.trimZeros,
        currencies,
      });
      setCfg((prev) => ({ ...prev, currencies }));
      Toast.success(t('保存成功'));
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
          {t('价格显示设置')}
        </Title>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>

      <Card>
        <Space vertical align="start" spacing={12} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Switch checked={cfg.enabled} onChange={(v) => setCfg((p) => ({ ...p, enabled: v }))} />
            <Text strong>{t('启用大额价格缩写')}</Text>
            <Text type="tertiary">
              {t('仅商品展示价生效（首页/分类/搜索/详情/推荐等）；购物车、结算、订单金额始终精确显示')}
            </Text>
          </Space>
          <Space align="center" spacing={12}>
            <Text>{t('缩写小数位')}</Text>
            <InputNumber
              min={0}
              max={4}
              precision={0}
              style={{ width: 90 }}
              value={cfg.decimals}
              onChange={(v) => setCfg((p) => ({ ...p, decimals: Number(v ?? 2) }))}
            />
            <Text>{t('自动去掉小数尾零')}</Text>
            <Switch
              checked={cfg.trimZeros}
              onChange={(v) => setCfg((p) => ({ ...p, trimZeros: v }))}
            />
            <Text type="tertiary">{t('如 100.00万 显示为 100万')}</Text>
          </Space>
          <Text type="tertiary">
            {t(
              '规则说明：金额达到「门槛」时按「除数」换算并带上「后缀」。例：人民币 1000000，配 ÷10000 后缀「万」显示 ¥100万；配 ÷1000000 后缀「百万」显示 ¥1百万。未配置的货币默认用 K/M/B，规则为空则该货币不缩写。',
            )}
          </Text>
        </Space>
      </Card>

      {currencyCodes.map((code) => {
        const meta = metaOf(code);
        const rules = cfg.currencies[code] || [];
        return (
          <Card
            key={code}
            title={`${meta.name} ${code}`}
            headerExtraContent={
              <Button type="danger" theme="borderless" size="small" onClick={() => removeCurrency(code)}>
                {t('删除币种')}
              </Button>
            }
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {rules.map((rule, index) => (
                <Space key={index} align="center" spacing={8}>
                  <Text>{t('金额 ≥')}</Text>
                  <InputNumber
                    min={1}
                    style={{ width: 150 }}
                    value={rule.at}
                    onChange={(v) => updateRule(code, index, { at: Number(v ?? 0) })}
                  />
                  <Text>{t('时，除以')}</Text>
                  <InputNumber
                    min={1}
                    style={{ width: 150 }}
                    value={rule.div}
                    onChange={(v) => updateRule(code, index, { div: Number(v ?? 0) })}
                  />
                  <Text>{t('后缀')}</Text>
                  <Input
                    style={{ width: 110 }}
                    value={rule.suffix}
                    maxLength={8}
                    onChange={(v) => updateRule(code, index, { suffix: v })}
                    placeholder={t('如 万 / K')}
                  />
                  <Button
                    type="danger"
                    theme="borderless"
                    icon={<IconDelete />}
                    onClick={() => removeRule(code, index)}
                  />
                </Space>
              ))}
              <div>
                <Button theme="borderless" icon={<IconPlus />} onClick={() => addRule(code)}>
                  {t('添加规则')}
                </Button>
              </div>
              <Text type="tertiary">
                {t('预览')}：
                {SAMPLE_AMOUNTS.map((amt) => `${amt.toLocaleString('en-US')} → ${previewFormat(cfg, code, amt)}`).join('；')}
              </Text>
            </div>
          </Card>
        );
      })}

      <Card>
        <Space align="center" spacing={12}>
          <Text strong>{t('添加币种')}</Text>
          <Select
            style={{ width: 240 }}
            placeholder={t('选择货币')}
            value={newCurrency || undefined}
            onChange={(v) => setNewCurrency(String(v || ''))}
            optionList={addableCurrencies.map((c) => ({
              value: c.code,
              label: `${c.name} ${c.code}（${c.symbol}）`,
            }))}
          />
          <Button type="primary" disabled={!newCurrency} onClick={addCurrency}>
            {t('添加')}
          </Button>
          <Text type="tertiary">{t('添加后按该货币的本地习惯预填规则，可再修改')}</Text>
        </Space>
      </Card>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>
    </div>
  );
}
