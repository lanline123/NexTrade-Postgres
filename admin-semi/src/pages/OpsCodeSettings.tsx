/**
 * 运营代码与验证（设置 → SEO →「运营代码与验证」，论坛帖 477914 批次）。
 *
 * 平台管理员专属（后端端点挂 platform_only，非平台账号调用 403）：
 * - 站长验证（我方新增）：Google/Bing/百度/360/搜狗/神马六平台的 meta
 *   验证码 + 单个验证文件（文件名 + 内容，PC 前台在域名根路径托管，
 *   如 https://域名/baidu_verify_xxx.html）。PC 首页服务端渲染时输出
 *   对应 meta，改完保存后 PC 端缓存期内（≤5 分钟）生效。
 * - 第三方代码（我方新增）：headCode（头部代码）/ bodyCode（底部
 *   代码）两个 JS 代码位，统计代码（百度统计/CNZZ/GA 等）粘 bodyCode。
 *   代码在 PC 与 H5 全站页面原样执行——仅粘贴可信来源代码。
 * 客服 JS 代码位不在本页：在「客服设置」页（原版字段 kefuJavascript /
 * wapKefuJavascript，与客服类型并行生效）。
 *
 * 存储：config 表 opsCodeSettings 一行 JSON（GET/POST
 * /setting/config/opsCodeSettings | saveOpsCode）；全部留空 = 前台零
 * 注入零影响。保存经后端严格校验：验证文件名非法、或文件名与内容
 * 只填其一会被明确拒绝。
 */
import { useCallback, useEffect, useState } from 'react';
import { Button, Input, Spin, TextArea, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';

interface OpsCodeSettings {
  verifyGoogle: string;
  verifyBing: string;
  verifyBaidu: string;
  verify360: string;
  verifySogou: string;
  verifyShenma: string;
  verifyFileName: string;
  verifyFileContent: string;
  headCode: string;
  bodyCode: string;
}

const DEFAULTS: OpsCodeSettings = {
  verifyGoogle: '',
  verifyBing: '',
  verifyBaidu: '',
  verify360: '',
  verifySogou: '',
  verifyShenma: '',
  verifyFileName: '',
  verifyFileContent: '',
  headCode: '',
  bodyCode: '',
};

const sectionStyle: React.CSSProperties = {
  background: '#fff',
  border: '1px solid #e4e7ec',
  borderRadius: 0,
  padding: '16px 20px',
  marginBottom: 16,
};
const rowStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 12,
  padding: '10px 0',
  borderBottom: '1px solid #f0f1f2',
};
const labelStyle: React.CSSProperties = { width: 180, color: '#444', flex: '0 0 auto' };
const hintStyle: React.CSSProperties = { color: '#8a8f99', fontSize: 12 };
const warnStyle: React.CSSProperties = { color: '#b42318', fontSize: 12 };

const VERIFY_FIELDS = [
  { key: 'verifyGoogle', label: 'Google', hint: 'Search Console「HTML 标签」给出的 content 值（google-site-verification）' },
  { key: 'verifyBing', label: 'Bing', hint: 'Bing 网站管理员「XML 文件/元标记」给出的 content 值（msvalidate.01）' },
  { key: 'verifyBaidu', label: '百度', hint: '百度搜索资源平台「HTML 标签验证」给出的 content 值（baidu-site-verification）' },
  { key: 'verify360', label: '360', hint: '360 站长平台给出的 content 值（360-site-verification）' },
  { key: 'verifySogou', label: '搜狗', hint: '搜狗站长平台给出的 content 值（sogou_site_verification）' },
  { key: 'verifyShenma', label: '神马', hint: '神马站长平台给出的 content 值（shenma-site-verification）' },
] as const;

/** 与后端 is_valid_verify_file_name 同口径的前端先行校验。 */
function verifyFileNameError(name: string): string | null {
  if (!name) return null;
  if (name.includes('/') || name.includes('\\') || name.includes('..')) {
    return '验证文件名不能包含路径';
  }
  if (!/^[A-Za-z0-9._-]+$/.test(name)) return '验证文件名只能含字母、数字、点、下划线、横线';
  const lower = name.toLowerCase();
  if (!(lower.endsWith('.html') || lower.endsWith('.txt')) || name.length <= 5) {
    return '验证文件名须以 .html 或 .txt 结尾';
  }
  return null;
}

export default function OpsCodeSettings() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [settings, setSettings] = useState<OpsCodeSettings>(DEFAULTS);

  const load = useCallback(async () => {
    try {
      const data = await apiGet<Partial<OpsCodeSettings>>('/setting/config/opsCodeSettings');
      setSettings({ ...DEFAULTS, ...data });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载运营代码与验证配置失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const patch = (next: Partial<OpsCodeSettings>) => setSettings((prev) => ({ ...prev, ...next }));

  const save = async () => {
    const nameError = verifyFileNameError(settings.verifyFileName.trim());
    if (nameError) {
      Toast.error({ content: nameError });
      return;
    }
    const hasName = settings.verifyFileName.trim() !== '';
    const hasContent = settings.verifyFileContent.trim() !== '';
    if (hasName !== hasContent) {
      Toast.error({ content: '验证文件名与验证文件内容必须同时填写（或同时留空）' });
      return;
    }
    setSaving(true);
    try {
      await apiPost('/setting/config/saveOpsCode', settings);
      Toast.success({ content: '已保存，PC/H5 将在缓存期内生效（≤5 分钟）' });
    } catch (e) {
      Toast.error({ content: (e as Error).message || '保存失败' });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 40, textAlign: 'center' }}>
        <Spin />
      </div>
    );
  }

  return (
    <div>
      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>站长验证（meta 标签）</h3>
        <p style={hintStyle}>
          在各站长平台添加站点后选择「HTML 标签验证」，把平台给出的 meta 标签 content 值填到对应行；
          只填值本身，不要粘整段 &lt;meta&gt; 标签。保存后 PC 首页源码 &lt;head&gt; 中会出现对应 meta。
        </p>
        {VERIFY_FIELDS.map((field) => (
          <div style={rowStyle} key={field.key}>
            <span style={labelStyle}>{field.label}</span>
            <Input
              value={settings[field.key]}
              style={{ width: 420 }}
              placeholder="粘贴平台给出的验证码值"
              onChange={(v) => patch({ [field.key]: v } as Partial<OpsCodeSettings>)}
            />
            <span style={hintStyle}>{field.hint}</span>
          </div>
        ))}
      </div>

      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>站长验证（验证文件）</h3>
        <p style={hintStyle}>
          平台要求把验证文件放在域名根目录时用这一组：填写平台给出的文件名与文件内容，
          系统会在 https://你的域名/文件名 直接返回该内容（仅 PC 域名根路径，单个文件）。
          文件名只能是单个文件名（字母/数字/._-），以 .html 或 .txt 结尾；文件名与内容须同时填写或同时留空。
        </p>
        <div style={rowStyle}>
          <span style={labelStyle}>验证文件名</span>
          <Input
            value={settings.verifyFileName}
            style={{ width: 420 }}
            placeholder="如 baidu_verify_xxxx.html"
            onChange={(v) => patch({ verifyFileName: v })}
          />
        </div>
        <div style={{ ...rowStyle, alignItems: 'flex-start' }}>
          <span style={labelStyle}>验证文件内容</span>
          <TextArea
            value={settings.verifyFileContent}
            style={{ width: 640 }}
            autosize={{ minRows: 3, maxRows: 8 }}
            placeholder="粘贴平台验证文件的完整内容"
            onChange={(v) => patch({ verifyFileContent: v })}
          />
        </div>
      </div>

      <div style={sectionStyle}>
        <h3 style={{ margin: '0 0 8px' }}>第三方代码</h3>
        <p style={warnStyle}>
          警告：粘贴到这里的代码将以原始形态直接在商城 PC 与 H5 的全站页面中执行（等同于站点级脚本权限）。
          只粘贴来自可信服务商（统计、客服、广告平台）的官方代码；填写错误或来源不可信的代码可能拖慢页面、
          窃取访客数据或破坏页面功能，保存前请确认来源。本页仅平台管理员可见可改。
        </p>
        <div style={{ ...rowStyle, alignItems: 'flex-start' }}>
          <span style={labelStyle}>头部代码（headCode）</span>
          <TextArea
            value={settings.headCode}
            style={{ width: 640 }}
            autosize={{ minRows: 4, maxRows: 12 }}
            placeholder={'粘贴需要尽早执行的代码，如 <script src="..."></script>'}
            onChange={(v) => patch({ headCode: v })}
          />
          <span style={hintStyle}>在页面最前部执行（PC 为服务端渲染进首屏 HTML，H5 为启动后立即注入）</span>
        </div>
        <div style={{ ...rowStyle, alignItems: 'flex-start' }}>
          <span style={labelStyle}>底部代码（bodyCode）</span>
          <TextArea
            value={settings.bodyCode}
            style={{ width: 640 }}
            autosize={{ minRows: 4, maxRows: 12 }}
            placeholder={'统计代码粘这里，如百度统计 / CNZZ / Google Analytics 的 <script> 代码'}
            onChange={(v) => patch({ bodyCode: v })}
          />
          <span style={hintStyle}>在页面底部执行，统计类代码推荐放这里，不阻塞页面渲染</span>
        </div>
        <p style={hintStyle}>
          第三方客服的 JS 代码请到「客服设置」页填写（PC 端 / 移动端两份，与客服类型并行生效）；全部留空即不注入任何代码。
        </p>
      </div>

      <Button theme="solid" type="primary" loading={saving} onClick={() => void save()}>
        保存设置
      </Button>
    </div>
  );
}
