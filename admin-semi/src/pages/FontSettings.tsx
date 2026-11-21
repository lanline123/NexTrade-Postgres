import { useEffect, useMemo, useState } from 'react';
import { Button, Card, Input, Select, Space, Spin, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import {
  DEFAULT_FONT_SETTINGS,
  FONT_REGISTRY,
  buildFontStack,
  ensureFontCss,
  applyAdminFont,
  type CustomFont,
  type FontSettingsConfig,
} from '../fontRegistry';

const { Text, Title } = Typography;

/**
 * 网站字体设置（设置 → 商城设置 → 网站字体设置）。
 *
 * 前台字体按语言分别配置（PC locale code），另有全站默认与后台（admin）
 * 字体。配置存后端 config code `fontSettings`，前台通过
 * GET /api/common/config/fontSettings 读取并注入字体栈。字体文件自托管
 * （主站 /fonts/ 切片 CSS），注册表见 src/fontRegistry.ts。
 */

const LOCALES: Array<{ code: string; label: string; preview: string }> = [
  { code: 'zh-CN', label: '简体中文', preview: '万物新生 · 动力电池回收装备 0123456789' },
  { code: 'zh-Hant', label: '繁体中文', preview: '萬物新生 · 動力電池回收裝備 0123456789' },
  { code: 'en-US', label: '英语', preview: 'Battery Recycling Equipment 0123456789' },
  { code: 'fr', label: '法语', preview: 'Équipement de recyclage de batteries 0123456789' },
  { code: 'de', label: '德语', preview: 'Batterie-Recycling Anlagen 0123456789' },
  { code: 'es', label: '西班牙语', preview: 'Equipos de reciclaje de baterías 0123456789' },
  { code: 'it', label: '意大利语', preview: 'Impianti di riciclo batterie 0123456789' },
  { code: 'pt', label: '葡萄牙语', preview: 'Equipamentos de reciclagem 0123456789' },
  { code: 'ru', label: '俄语', preview: 'Оборудование для переработки 0123456789' },
  { code: 'ja', label: '日语', preview: 'バッテリー回收設備 0123456789' },
  { code: 'ko', label: '韩语', preview: '배터리 재활용 장비 0123456789' },
];

/** 与后端 recommended_font_id 一致的出厂推荐。 */
function recommendedFor(code: string): string {
  if (code === 'ko') return 'noto-sans-kr';
  if (code === 'ja') return 'noto-sans-jp';
  if (code === 'ru') return 'roboto';
  if (['en-US', 'fr', 'de', 'es', 'it', 'pt'].includes(code)) return 'inter';
  return 'alibaba-puhuiti';
}

export default function FontSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [cfg, setCfg] = useState<FontSettingsConfig>(DEFAULT_FONT_SETTINGS);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiGet<Record<string, any>>('/setting/config/fontSettings');
        if (data && typeof data === 'object') {
          setCfg({
            version: 1,
            storefront: {
              default: { fontId: data.storefront?.default?.fontId || 'alibaba-puhuiti' },
              locales: { ...DEFAULT_FONT_SETTINGS.storefront.locales, ...(data.storefront?.locales || {}) },
            },
            admin: { fontId: data.admin?.fontId || 'alibaba-puhuiti' },
            customFonts: Array.isArray(data.customFonts) ? data.customFonts : [],
          });
        }
      } catch {
        Toast.error('字体配置加载失败，已显示内置默认');
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const fontOptions = useMemo(() => {
    const opts = FONT_REGISTRY.map((f) => ({ value: f.id, label: f.name }));
    for (const c of cfg.customFonts) {
      if (c.id && !opts.some((o) => o.value === c.id)) {
        opts.push({ value: c.id, label: `${c.name || c.id}（自备）` });
      }
    }
    return opts;
  }, [cfg.customFonts]);

  // 预览需要所选字体真实加载：把当前配置涉及的字体 CSS 全部按需插入
  useEffect(() => {
    const ids = new Set<string>([
      cfg.storefront.default.fontId,
      cfg.admin.fontId,
      ...Object.values(cfg.storefront.locales).map((s) => s.fontId),
    ]);
    for (const id of ids) {
      const def = FONT_REGISTRY.find((f) => f.id === id);
      if (def?.cssUrl) ensureFontCss([def.cssUrl]);
      const custom = cfg.customFonts.find((c) => c.id === id);
      if (custom?.cssUrl) ensureFontCss([custom.cssUrl]);
    }
  }, [cfg]);

  const setLocaleFont = (code: string, fontId: string) =>
    setCfg((c) => ({
      ...c,
      storefront: { ...c.storefront, locales: { ...c.storefront.locales, [code]: { fontId } } },
    }));

  const setCustom = (idx: number, patch: Partial<CustomFont>) =>
    setCfg((c) => ({
      ...c,
      customFonts: c.customFonts.map((f, i) => (i === idx ? { ...f, ...patch } : f)),
    }));

  const save = async () => {
    setSaving(true);
    try {
      await apiPost('/setting/config/saveFontSettings', cfg);
      applyAdminFont(cfg);
      Toast.success('已保存：前台下次打开页面生效（缓存 ≤300 秒），后台字体已立即应用');
    } catch {
      Toast.error('保存失败，请重试');
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

  const previewStyle = (fontId: string): React.CSSProperties => ({
    fontFamily: buildFontStack(fontId, cfg.customFonts),
    fontSize: 15,
    color: '#1c1f23',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  });

  return (
    <div style={{ padding: 20, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Card>
        <Title heading={5} style={{ marginBottom: 4 }}>前台字体（按语言）</Title>
        <Text type="tertiary">
          每个语言可单独选字体；混排时缺字自动回落全站默认字体（普惠体）与系统字体。未单独设置的语言走全站默认。
        </Text>
        <div style={{ marginTop: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <span style={{ width: 110, fontWeight: 600 }}>全站默认</span>
            <Select
              value={cfg.storefront.default.fontId}
              optionList={fontOptions}
              style={{ width: 260 }}
              onChange={(v) =>
                setCfg((c) => ({ ...c, storefront: { ...c.storefront, default: { fontId: String(v) } } }))
              }
            />
            <span style={previewStyle(cfg.storefront.default.fontId)}>万物新生 · Battery Recycling 0123456789</span>
          </div>
          {LOCALES.map(({ code, label, preview }) => {
            const fontId = cfg.storefront.locales[code]?.fontId || cfg.storefront.default.fontId;
            const rec = recommendedFor(code);
            return (
              <div key={code} style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <span style={{ width: 110 }}>
                  {label} <Text type="tertiary">({code})</Text>
                </span>
                <Select
                  value={fontId}
                  optionList={fontOptions}
                  style={{ width: 260 }}
                  onChange={(v) => setLocaleFont(code, String(v))}
                />
                {fontId === rec && <Tag color="green">推荐</Tag>}
                <span style={previewStyle(fontId)}>{preview}</span>
              </div>
            );
          })}
        </div>
      </Card>

      <Card>
        <Title heading={5} style={{ marginBottom: 4 }}>后台字体</Title>
        <Text type="tertiary">管理后台界面使用的字体（不按语言区分），保存后立即对当前会话生效。</Text>
        <div style={{ marginTop: 16, display: 'flex', alignItems: 'center', gap: 12 }}>
          <span style={{ width: 110, fontWeight: 600 }}>后台字体</span>
          <Select
            value={cfg.admin.fontId}
            optionList={fontOptions}
            style={{ width: 260 }}
            onChange={(v) => setCfg((c) => ({ ...c, admin: { fontId: String(v) } }))}
          />
          <span style={previewStyle(cfg.admin.fontId)}>后台管理 · 商品管理 0123456789</span>
        </div>
      </Card>

      <Card>
        <Title heading={5} style={{ marginBottom: 4 }}>自备字体（可选）</Title>
        <Text type="tertiary">
          如需接入注册表之外的字体，把字体 CSS 托管到任意 https 地址后登记在这里，其 id 即可在上方各语言中选用。
          id 仅限小写字母、数字与连字符；family 与 CSS 地址含特殊符号的条目会被服务端丢弃。
        </Text>
        <div style={{ marginTop: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
          {cfg.customFonts.map((f, i) => (
            <Space key={i} spacing={8} wrap>
              <Input value={f.id} placeholder="id（如 myfont1）" style={{ width: 140 }}
                onChange={(v) => setCustom(i, { id: v })} />
              <Input value={f.name} placeholder="显示名" style={{ width: 150 }}
                onChange={(v) => setCustom(i, { name: v })} />
              <Input value={f.family} placeholder="font-family 名" style={{ width: 170 }}
                onChange={(v) => setCustom(i, { family: v })} />
              <Input value={f.cssUrl} placeholder="https://…/font.css" style={{ width: 300 }}
                onChange={(v) => setCustom(i, { cssUrl: v })} />
              <Button type="danger" theme="borderless"
                onClick={() => setCfg((c) => ({ ...c, customFonts: c.customFonts.filter((_, j) => j !== i) }))}>
                删除
              </Button>
            </Space>
          ))}
          <div>
            <Button
              disabled={cfg.customFonts.length >= 8}
              onClick={() =>
                setCfg((c) => ({ ...c, customFonts: [...c.customFonts, { id: '', name: '', family: '', cssUrl: '' }] }))
              }
            >
              添加自备字体
            </Button>
          </div>
        </div>
      </Card>

      <div>
        <Button theme="solid" type="primary" loading={saving} onClick={save}>
          保存设置
        </Button>
      </div>
    </div>
  );
}
