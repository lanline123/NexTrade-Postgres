import { useEffect, useState, type CSSProperties } from 'react';
import { Button, Card, Space, Spin, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import GalleryPickerModal from '../components/GalleryPickerModal';
import ColorField from '../components/ColorField';
import { PcBuyButtonVisual, resolvePcBuyBtn } from './decoratePc/previews/shared/PcBuyButton';

const { Text, Title } = Typography;

/**
 * 全站购买按钮样式设置（设置 → 商城设置 → 购买按钮样式设置）。
 *
 * 商品类模块（商品/商品分组/拼团/秒杀/搜索结果）的购买按钮统一枚举 0–10，
 * 模块可单独选样式，也可设「跟随全局」用这里的配置。配置存后端 config code
 * `buyStyle`，前台通过 GET /api/common/config/buyStyle 读取。自定义图片（样式 10）
 * 从素材库选图（支持 SVG），模块自带图标优先、全局图标兜底。
 *
 * 样式选择为单横排图标块（用户 2026-11-19 口径）：每档渲染该样式的真实按钮
 * 样貌小预览，一排选完，不用下拉。图标颜色默认 = PC 端主题色：主题色与 PC
 * 前台同源——后台「主题风格」themeStyle 的 `--general`（pc-semi 里
 * `--pc-primary: var(--general)`，SSR 注入 :root 的同一份数据）。未显式保存过
 * 颜色（iconColor 为空）时把主题色初始化进表单并按此保存，预览与保存口径
 * 一致；已显式保存过颜色的老配置原样加载、不被主题色覆盖；「重置」回到
 * 当前 PC 主题色。
 */

interface BuyStyleConfig {
  style: number;
  customIcon: { picUrl: string; picThumb: string };
  iconColor: string;
}

const STYLE_OPTIONS: Array<{ value: number; label: string; short: string }> = [
  { value: 0, label: '不显示购买按钮', short: '不显示' },
  { value: 1, label: '购物车（描边）', short: '购物车·描边' },
  { value: 2, label: '购物车（实心）', short: '购物车·实心' },
  { value: 3, label: '加号（描边）', short: '加号·描边' },
  { value: 4, label: '加号（实心）', short: '加号·实心' },
  { value: 5, label: '“购买”（描边）', short: '购买·描边' },
  { value: 6, label: '“购买”（实心）', short: '购买·实心' },
  { value: 7, label: '“马上抢”（描边）', short: '马上抢·描边' },
  { value: 8, label: '“马上抢”（实心）', short: '马上抢·实心' },
  { value: 9, label: '圆圈购物车', short: '圆圈购物车' },
  { value: 10, label: '自定义图片', short: '自定义图片' },
];

const DEFAULT_CFG: BuyStyleConfig = {
  style: 1,
  customIcon: { picUrl: '', picThumb: '' },
  iconColor: '',
};

/** PC 主题色兜底（与 pc-semi --pc-primary 的回落值、编辑器预览兜底一致）。 */
const FALLBACK_THEME_COLOR = '#65c4aa';

/**
 * 从后台「主题风格」themeStyle 取 PC 主题色。PC 前台的主色变量 --pc-primary
 * 直接消费 --general，故默认色取 --general；个别存量主题只写了 --main-bg 时
 * 以其兜底。解析口径与 PC 装修编辑器画布一致（themeStyle 可为字符串化 JSON；
 * 值含危险字符直接弃用，防 CSS 注入），最终必须是合法 hex 才采用。
 */
function pickPcThemeColor(data: { themeStyle?: string | Record<string, any> } | null | undefined): string {
  try {
    const parsed = typeof data?.themeStyle === 'string' ? JSON.parse(data.themeStyle) : data?.themeStyle;
    if (!parsed || typeof parsed !== 'object') return FALLBACK_THEME_COLOR;
    for (const key of ['--general', '--main-bg']) {
      const raw = (parsed as Record<string, unknown>)[key];
      if (typeof raw !== 'string' && typeof raw !== 'number') continue;
      const text = String(raw).trim();
      if (!text || /[;{}<>\\]/.test(text)) continue;
      if (/^#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(text)) return text;
    }
  } catch {
    /* 解析失败走兜底色 */
  }
  return FALLBACK_THEME_COLOR;
}

/** 样式 0（不显示）在横排里的占位图形：灰色禁选圈，与「无按钮」语义对应。 */
function HiddenStyleGlyph() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="9" stroke="#b6bcc4" strokeWidth="2" />
      <line x1="5.8" y1="5.8" x2="18.2" y2="18.2" stroke="#b6bcc4" strokeWidth="2" />
    </svg>
  );
}

export default function BuyStyleSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [cfg, setCfg] = useState<BuyStyleConfig>(DEFAULT_CFG);
  const [themeColor, setThemeColor] = useState(FALLBACK_THEME_COLOR);

  useEffect(() => {
    (async () => {
      // 主题色与购买样式并行读：主题色读失败不阻断本页（走兜底色）
      const [themeRes, cfgRes] = await Promise.allSettled([
        apiGet<{ themeStyle?: string | Record<string, any> }>('/setting/config/themeStyleSettings'),
        apiGet<Record<string, any>>('/setting/config/buyStyleSettings'),
      ]);
      const theme = themeRes.status === 'fulfilled' ? pickPcThemeColor(themeRes.value) : FALLBACK_THEME_COLOR;
      setThemeColor(theme);
      if (cfgRes.status === 'fulfilled') {
        const data = cfgRes.value;
        if (data && typeof data === 'object') {
          const icon = (data.customIcon || {}) as Record<string, any>;
          const savedColor = String(data.iconColor || '').trim();
          setCfg({
            style: Number.isFinite(Number(data.style)) ? Number(data.style) : 1,
            customIcon: {
              picUrl: String(icon.picUrl || ''),
              picThumb: String(icon.picThumb || ''),
            },
            // 未显式设色（空串）→ 默认色初始化为 PC 主题色；
            // 已保存过颜色的老配置原样保留，不被覆盖
            iconColor: savedColor || theme,
          });
        } else {
          setCfg((p) => ({ ...p, iconColor: p.iconColor || theme }));
        }
      } else {
        Toast.error(t('加载购买按钮样式设置失败'));
      }
      setLoading(false);
    })();
  }, [t]);

  // 生效颜色：表单值为空的瞬间（取色器清空）也按主题色渲染与保存，口径一致
  const effectiveColor = cfg.iconColor.trim() || themeColor;
  const isThemeColor = effectiveColor.toLowerCase() === themeColor.toLowerCase();
  // 选中态浅底：仅 6 位 hex 可拼 alpha 后缀，其余形态走固定浅绿兜底
  const selectedBg = /^#[0-9a-f]{6}$/i.test(themeColor) ? `${themeColor}14` : 'rgba(101, 196, 170, 0.08)';

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveBuyStyle', {
        style: cfg.style,
        customIcon: cfg.customIcon,
        iconColor: effectiveColor,
      });
      Toast.success(t('保存成功'));
    } catch {
      Toast.error(t('保存失败'));
    } finally {
      setSaving(false);
    }
  };

  // 实时预览走与前台/编辑器相同的解析链（-1 跟随全局 → 本页配置）
  const preview = resolvePcBuyBtn(-1, {
    global: { style: cfg.style, customIcon: cfg.customIcon, iconColor: effectiveColor },
  });
  const iconUrl = cfg.customIcon.picThumb || cfg.customIcon.picUrl;

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin size="large" />
      </div>
    );
  }

  // 预览壳着色口径与 PC 前台一致：图标走 color（iconColor），实心文案款底色走
  // --pc-primary（= PC 主题色本身，前台不随 iconColor 变）
  const shellVars = { '--pc-primary': themeColor } as CSSProperties;

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Title heading={4} style={{ margin: 0 }}>
          {t('购买按钮样式设置')}
        </Title>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>

      <Card>
        <Space vertical align="start" spacing={16} style={{ width: '100%' }}>
          <div style={{ width: '100%' }}>
            <Text strong>{t('全局购买按钮样式')}</Text>
            <div
              style={{
                display: 'flex',
                flexWrap: 'nowrap',
                gap: 10,
                overflowX: 'auto',
                padding: '12px 2px 8px',
              }}
            >
              {STYLE_OPTIONS.map((o) => {
                const selected = cfg.style === o.value;
                return (
                  <button
                    key={o.value}
                    type="button"
                    title={t(o.label)}
                    aria-pressed={selected}
                    onClick={() => setCfg((p) => ({ ...p, style: o.value }))}
                    style={{
                      flex: '0 0 auto',
                      width: 88,
                      padding: '10px 4px 8px',
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      gap: 8,
                      background: selected ? selectedBg : '#fff',
                      border: `1px solid ${selected ? themeColor : '#e3e5e8'}`,
                      boxShadow: selected ? `0 0 0 1px ${themeColor}` : undefined,
                      borderRadius: 8,
                      cursor: 'pointer',
                      font: 'inherit',
                    }}
                  >
                    <span
                      style={{
                        ...shellVars,
                        height: 34,
                        display: 'inline-flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: effectiveColor,
                      }}
                    >
                      {o.value === 0 ? (
                        <HiddenStyleGlyph />
                      ) : (
                        <PcBuyButtonVisual style={o.value} customIcon={o.value === 10 ? cfg.customIcon : null} />
                      )}
                    </span>
                    <span
                      style={{
                        fontSize: 12,
                        lineHeight: 1.35,
                        color: selected ? '#1f2329' : '#5f6570',
                        fontWeight: selected ? 600 : 400,
                        textAlign: 'center',
                      }}
                    >
                      {t(o.short)}
                    </span>
                  </button>
                );
              })}
            </div>
            {cfg.style === 0 ? (
              <Text type="danger">{t('选「不显示」时，所有设为跟随全局的模块都不显示购买按钮')}</Text>
            ) : null}
          </div>

          {cfg.style === 10 ? (
            <Space align="center" spacing={12}>
              <Text strong>{t('自定义图标')}</Text>
              {iconUrl ? (
                <span
                  style={{
                    width: 44,
                    height: 44,
                    border: '1px solid #e3e5e8',
                    borderRadius: 6,
                    display: 'inline-flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    background: '#fff',
                  }}
                >
                  <img src={iconUrl} alt="" style={{ maxWidth: 36, maxHeight: 36, objectFit: 'contain' }} />
                </span>
              ) : null}
              <Button onClick={() => setPickerOpen(true)}>{t('从素材库选择')}</Button>
              {iconUrl ? (
                <Button
                  type="danger"
                  theme="borderless"
                  onClick={() => setCfg((p) => ({ ...p, customIcon: { picUrl: '', picThumb: '' } }))}
                >
                  {t('删除')}
                </Button>
              ) : null}
              <Text type="tertiary">{t('支持 PNG/JPG/SVG；模块自带图标优先，此处为兜底')}</Text>
            </Space>
          ) : null}

          <Space align="center" spacing={12}>
            <Text strong>{t('图标颜色')}</Text>
            <ColorField
              value={cfg.iconColor}
              onChange={(v) => setCfg((p) => ({ ...p, iconColor: v }))}
              resetValue={themeColor}
            />
            <Text>{effectiveColor}</Text>
            {isThemeColor ? <Text type="tertiary">{t('（PC 主题色）')}</Text> : null}
            <Text type="tertiary">{t('默认为 PC 端主题色，可自定义，重置即恢复主题色；仅对图标类样式（1–4、9）生效')}</Text>
          </Space>

          <Text type="tertiary">
            {t(
              '说明：商品、商品分组、拼团、秒杀、搜索结果等模块的购买按钮共用同一套样式。模块在装修编辑器里选「跟随全局」时使用这里的设置；模块单独选了样式则以模块为准。',
            )}
          </Text>
        </Space>
      </Card>

      <Card title={t('效果预览')}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 16,
            padding: '10px 14px',
            border: '1px solid #eef0f3',
            borderRadius: 8,
            maxWidth: 420,
            background: '#fff',
          }}
        >
          <div
            style={{
              width: 64,
              height: 64,
              borderRadius: 6,
              background: 'linear-gradient(135deg,#f2f4f7,#e6eaef)',
              flexShrink: 0,
            }}
          />
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 13, color: '#333', marginBottom: 6 }}>{t('示例商品名称')}</div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ color: '#ff3f4f', fontWeight: 600, fontSize: 15 }}>¥6.5万</span>
              {preview.show ? (
                <span style={{ ...shellVars, display: 'inline-flex', color: effectiveColor }}>
                  <PcBuyButtonVisual style={preview.style} customIcon={preview.customIcon} />
                </span>
              ) : (
                <Text type="tertiary">{t('（不显示）')}</Text>
              )}
            </div>
          </div>
        </div>
      </Card>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>

      <GalleryPickerModal
        visible={pickerOpen}
        multiple={false}
        limit={1}
        onCancel={() => setPickerOpen(false)}
        onConfirm={(urls) => {
          const url = urls[0] || '';
          setCfg((p) => ({ ...p, customIcon: { picUrl: url, picThumb: url } }));
          setPickerOpen(false);
        }}
      />
    </div>
  );
}
