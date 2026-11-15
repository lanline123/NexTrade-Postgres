import { useEffect, useState } from 'react';
import { Button, Card, Select, Space, Spin, Toast, Typography } from '@douyinfe/semi-ui';
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
 */

interface BuyStyleConfig {
  style: number;
  customIcon: { picUrl: string; picThumb: string };
  iconColor: string;
}

const STYLE_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 0, label: '不显示购买按钮' },
  { value: 1, label: '购物车（描边）' },
  { value: 2, label: '购物车（实心）' },
  { value: 3, label: '加号（描边）' },
  { value: 4, label: '加号（实心）' },
  { value: 5, label: '“购买”（描边）' },
  { value: 6, label: '“购买”（实心）' },
  { value: 7, label: '“马上抢”（描边）' },
  { value: 8, label: '“马上抢”（实心）' },
  { value: 9, label: '圆圈购物车' },
  { value: 10, label: '自定义图片' },
];

const DEFAULT_CFG: BuyStyleConfig = {
  style: 1,
  customIcon: { picUrl: '', picThumb: '' },
  iconColor: '',
};

export default function BuyStyleSettings() {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [cfg, setCfg] = useState<BuyStyleConfig>(DEFAULT_CFG);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<Record<string, any>>('/setting/config/buyStyleSettings');
        if (data && typeof data === 'object') {
          const icon = (data.customIcon || {}) as Record<string, any>;
          setCfg({
            style: Number.isFinite(Number(data.style)) ? Number(data.style) : 1,
            customIcon: {
              picUrl: String(icon.picUrl || ''),
              picThumb: String(icon.picThumb || ''),
            },
            iconColor: String(data.iconColor || ''),
          });
        }
      } catch {
        Toast.error(t('加载购买按钮样式设置失败'));
      } finally {
        setLoading(false);
      }
    })();
  }, [t]);

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveBuyStyle', {
        style: cfg.style,
        customIcon: cfg.customIcon,
        iconColor: cfg.iconColor,
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
    global: { style: cfg.style, customIcon: cfg.customIcon, iconColor: cfg.iconColor },
  });
  const iconUrl = cfg.customIcon.picThumb || cfg.customIcon.picUrl;

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
          {t('购买按钮样式设置')}
        </Title>
        <Button type="primary" theme="solid" loading={saving} onClick={save}>
          {t('保存')}
        </Button>
      </div>

      <Card>
        <Space vertical align="start" spacing={16} style={{ width: '100%' }}>
          <Space align="center" spacing={12}>
            <Text strong>{t('全局购买按钮样式')}</Text>
            <Select
              style={{ width: 260 }}
              value={cfg.style}
              onChange={(v) => setCfg((p) => ({ ...p, style: Number(v ?? 1) }))}
              optionList={STYLE_OPTIONS.map((o) => ({ value: o.value, label: t(o.label) }))}
            />
            {cfg.style === 0 ? (
              <Text type="danger">{t('选「不显示」时，所有设为跟随全局的模块都不显示购买按钮')}</Text>
            ) : null}
          </Space>

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
            <ColorField value={cfg.iconColor} onChange={(v) => setCfg((p) => ({ ...p, iconColor: v }))} resetValue="" />
            <Text>{cfg.iconColor || t('未设置（跟随各模块/主题色）')}</Text>
            <Text type="tertiary">{t('仅对图标类样式（1–4、9）生效')}</Text>
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
                <span style={{ display: 'inline-flex', color: 'var(--main-bg, #65c4aa)' }}>
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
