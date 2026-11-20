/**
 * AI 渠道与密钥配置页（我方新增）。
 * 渠道 = 一个上游服务；密钥只写不读（列表只回打码，留空 = 不修改）；
 * 平台的官方渠道对商家只读展示（不含任何密钥信息）。蜂鸟/火山渠道在 P1
 * 先行登记（双读生效），正式迁移在 P2。
 */
import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Card,
  Input,
  Modal,
  Select,
  Switch,
  Table,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import { apiDelete, apiGet, apiPost, apiPut } from '../api';

interface AiModel {
  modelId?: number;
  modelCode: string;
  displayName: string;
  capabilities: string;
  contextTokens: number;
  isEnabled: boolean;
}

interface AiChannel {
  channelId: number;
  name: string;
  kind: number;
  provider: string;
  baseUrl: string;
  extraKeyName: string;
  isEnabled: boolean;
  isDefault: boolean;
  keyMask: string;
  hasKey: boolean;
  extraKeyMask: string;
  hasExtraKey: boolean;
  official: boolean;
  models: AiModel[];
}

interface Preset {
  name: string;
  provider: string;
  baseUrl: string;
  models: { modelCode: string; displayName: string; capabilities: string }[];
}

const KIND_LABEL: Record<number, string> = {
  1: '官方渠道',
  2: 'OpenAI 兼容',
  3: '蜂鸟专有',
  4: '火山翻译',
};

const isPlatform = () => {
  const t = localStorage.getItem('adminType') || '';
  return t === '' || t === 'admin';
};

interface FormState {
  channelId: number;
  name: string;
  kind: number;
  provider: string;
  baseUrl: string;
  apiKey: string;
  extraKey: string;
  extraKeyName: string;
  models: AiModel[];
}

const emptyForm = (): FormState => ({
  channelId: 0,
  name: '',
  kind: 2,
  provider: '',
  baseUrl: '',
  apiKey: '',
  extraKey: '',
  extraKeyName: '',
  models: [],
});

export default function AiChannels() {
  const [channels, setChannels] = useState<AiChannel[]>([]);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [form, setForm] = useState<FormState>(emptyForm());
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<number>(0);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiGet<{ channels: AiChannel[]; presets: Preset[] }>('/ai/channels');
      setChannels(res.channels || []);
      setPresets(res.presets || []);
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载失败' });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const openCreate = (preset?: Preset) => {
    const f = emptyForm();
    if (preset) {
      f.name = preset.name;
      f.provider = preset.provider;
      f.baseUrl = preset.baseUrl;
      f.models = preset.models.map((m) => ({
        modelCode: m.modelCode,
        displayName: m.displayName,
        capabilities: m.capabilities,
        contextTokens: 0,
        isEnabled: true,
      }));
    }
    setForm(f);
    setModalOpen(true);
  };

  const openEdit = (ch: AiChannel) => {
    setForm({
      channelId: ch.channelId,
      name: ch.name,
      kind: ch.kind,
      provider: ch.provider,
      baseUrl: ch.baseUrl,
      apiKey: '',
      extraKey: '',
      extraKeyName: ch.extraKeyName,
      models: ch.models.map((m) => ({ ...m })),
    });
    setModalOpen(true);
  };

  const save = async () => {
    if (!form.name.trim()) {
      Toast.warning({ content: '请填写渠道名称' });
      return;
    }
    if (form.kind === 2 && !form.baseUrl.trim()) {
      Toast.warning({ content: '请填写接口地址（Base URL）' });
      return;
    }
    setSaving(true);
    try {
      const payload = {
        name: form.name.trim(),
        kind: form.kind,
        provider: form.provider,
        baseUrl: form.baseUrl.trim(),
        apiKey: form.apiKey || undefined,
        extraKey: form.extraKey || undefined,
        extraKeyName: form.extraKeyName || undefined,
        models: form.models
          .filter((m) => m.modelCode.trim())
          .map((m) => ({
            modelId: m.modelId,
            modelCode: m.modelCode.trim(),
            displayName: m.displayName,
            capabilities: m.capabilities,
            contextTokens: m.contextTokens,
            isEnabled: m.isEnabled,
          })),
      };
      if (form.channelId) {
        await apiPut(`/ai/channels/${form.channelId}`, payload);
      } else {
        await apiPost('/ai/channels', payload);
      }
      Toast.success({ content: '已保存（密钥加密存储）' });
      setModalOpen(false);
      void load();
    } catch (e) {
      Toast.error({ content: (e as Error).message || '保存失败' });
    } finally {
      setSaving(false);
    }
  };

  const test = async (channelId: number) => {
    setTesting(channelId);
    try {
      const res = await apiPost<{ ok: boolean; latencyMs: number; message: string }>(
        `/ai/channels/${channelId}/test`,
        {},
      );
      if (res.ok) {
        Toast.success({ content: `连接成功，延迟 ${res.latencyMs}ms` });
      } else {
        Toast.error({ content: `连接失败：${res.message}` });
      }
    } catch (e) {
      Toast.error({ content: (e as Error).message || '测试失败' });
    } finally {
      setTesting(0);
    }
  };

  const columns = [
    {
      title: '渠道',
      dataIndex: 'name',
      render: (_: unknown, ch: AiChannel) => (
        <span>
          {ch.name}{' '}
          {ch.official && <Tag color="blue">官方</Tag>}
          {ch.isDefault && <Tag color="green">默认</Tag>}
        </span>
      ),
    },
    {
      title: '类型',
      dataIndex: 'kind',
      render: (v: number) => KIND_LABEL[v] || v,
    },
    { title: '接口地址', dataIndex: 'baseUrl' },
    {
      title: '密钥',
      dataIndex: 'keyMask',
      render: (_: unknown, ch: AiChannel) =>
        ch.official ? (
          <span style={{ color: 'var(--semi-color-text-2)' }}>平台托管</span>
        ) : ch.hasKey ? (
          <span>
            {ch.keyMask}
            {ch.kind >= 3 && (
              <span style={{ color: 'var(--semi-color-text-2)' }}>
                {' '}
                / {ch.hasExtraKey ? ch.extraKeyMask : '未配置第二密钥'}
              </span>
            )}
          </span>
        ) : (
          <Tag color="red">未配置</Tag>
        ),
    },
    {
      title: '模型数',
      dataIndex: 'models',
      render: (ms: AiModel[]) => ms?.length ?? 0,
    },
    {
      title: '启用',
      dataIndex: 'isEnabled',
      render: (v: boolean, ch: AiChannel) =>
        ch.official ? (
          v ? '启用' : '停用'
        ) : (
          <Switch
            checked={v}
            onChange={async (checked) => {
              try {
                await apiPut(`/ai/channels/${ch.channelId}`, { isEnabled: checked });
                void load();
              } catch (e) {
                Toast.error({ content: (e as Error).message || '操作失败' });
              }
            }}
          />
        ),
    },
    {
      title: '操作',
      render: (_: unknown, ch: AiChannel) =>
        ch.official ? (
          <span style={{ color: 'var(--semi-color-text-2)' }}>只读</span>
        ) : (
          <span style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            <Button size="small" onClick={() => openEdit(ch)}>
              编辑
            </Button>
            {ch.kind === 2 && (
              <Button
                size="small"
                loading={testing === ch.channelId}
                onClick={() => void test(ch.channelId)}
              >
                测试连接
              </Button>
            )}
            {!ch.isDefault && (
              <Button
                size="small"
                onClick={async () => {
                  try {
                    await apiPost(`/ai/channels/${ch.channelId}/default`, {});
                    void load();
                  } catch (e) {
                    Toast.error({ content: (e as Error).message || '操作失败' });
                  }
                }}
              >
                设为默认
              </Button>
            )}
            <Button
              size="small"
              type="danger"
              onClick={() => {
                Modal.confirm({
                  title: '删除渠道',
                  content: `确定删除渠道「${ch.name}」吗？其密钥与模型将一并删除。`,
                  onOk: async () => {
                    await apiDelete(`/ai/channels/${ch.channelId}`);
                    void load();
                  },
                });
              }}
            >
              删除
            </Button>
          </span>
        ),
    },
  ];

  return (
    <div>
      <Card
        title="AI 渠道与密钥"
        headerExtraContent={
          <span style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <Select
              placeholder="从预设新建"
              style={{ width: 180 }}
              value={undefined}
              onChange={(v) => {
                const p = presets.find((x) => x.name === v);
                if (p) openCreate(p);
              }}
            >
              {presets.map((p) => (
                <Select.Option key={p.name} value={p.name}>
                  {p.name}
                </Select.Option>
              ))}
            </Select>
            <Button theme="solid" onClick={() => openCreate()}>
              新建渠道
            </Button>
          </span>
        }
      >
        <div style={{ color: 'var(--semi-color-text-2)', fontSize: 12, marginBottom: 10 }}>
          密钥经 AES-256-GCM 加密存储，只写不读：列表仅显示打码，编辑时留空表示不修改。
          {isPlatform()
            ? '平台在此页看不到任何商家的渠道与密钥。'
            : '你的渠道与密钥仅本店可见，平台完全不可见。'}
        </div>
        <Table
          columns={columns}
          dataSource={channels}
          rowKey="channelId"
          loading={loading}
          pagination={false}
        />
      </Card>

      <Modal
        title={form.channelId ? '编辑渠道' : '新建渠道'}
        visible={modalOpen}
        onCancel={() => setModalOpen(false)}
        width={760}
        className="ai-channel-modal"
        footer={
          <div className="ai-channel-footer">
            <Button theme="borderless" onClick={() => setModalOpen(false)}>
              取消
            </Button>
            <Button theme="solid" type="primary" loading={saving} onClick={() => void save()}>
              保存
            </Button>
          </div>
        }
      >
        <div className="ai-channel-form">
          <div className="ai-channel-section">
            <div className="ai-channel-section-title">基础信息</div>
            <div className="ai-channel-stack">
              <div className="ai-channel-row">
                <div className="ai-channel-field ai-channel-field-grow">
                  <div className="ai-channel-label">渠道名称</div>
                  <Input
                    placeholder="如 DeepSeek"
                    value={form.name}
                    onChange={(v) => setForm({ ...form, name: v })}
                  />
                </div>
                <div className="ai-channel-field ai-channel-field-kind">
                  <div className="ai-channel-label">渠道类型</div>
                  <Select
                    value={form.kind}
                    onChange={(v) => setForm({ ...form, kind: Number(v) })}
                    style={{ width: '100%' }}
                  >
                    {isPlatform() && <Select.Option value={1}>官方渠道（平台）</Select.Option>}
                    <Select.Option value={2}>OpenAI 兼容</Select.Option>
                    <Select.Option value={3}>蜂鸟专有</Select.Option>
                    <Select.Option value={4}>火山翻译</Select.Option>
                  </Select>
                </div>
              </div>
              {form.kind === 2 && (
                <div className="ai-channel-field">
                  <div className="ai-channel-label">接口地址（Base URL）</div>
                  <Input
                    className="ai-channel-mono"
                    placeholder="如 https://api.deepseek.com"
                    value={form.baseUrl}
                    onChange={(v) => setForm({ ...form, baseUrl: v })}
                  />
                </div>
              )}
              {form.kind !== 1 && (
                <div className="ai-channel-field">
                  <div className="ai-channel-label">
                    {form.kind === 3
                      ? '蜂鸟 Authorization'
                      : form.kind === 4
                        ? '火山 AccessKey ID'
                        : 'API 密钥'}
                  </div>
                  <Input
                    mode="password"
                    className="ai-channel-mono"
                    placeholder={form.channelId ? '已配置，留空表示不修改' : '请输入'}
                    value={form.apiKey}
                    onChange={(v) => setForm({ ...form, apiKey: v })}
                  />
                  {!form.channelId && (
                    <div className="ai-channel-hint">提交后加密存储，保存后不可查看原文。</div>
                  )}
                </div>
              )}
              {form.kind === 3 && (
                <div className="ai-channel-field">
                  <div className="ai-channel-label">蜂鸟 Project ID（第二凭据名）</div>
                  <Input
                    placeholder="请输入"
                    value={form.extraKeyName}
                    onChange={(v) => setForm({ ...form, extraKeyName: v })}
                  />
                </div>
              )}
              {form.kind === 4 && (
                <div className="ai-channel-field">
                  <div className="ai-channel-label">火山 Secret Access Key（第二凭据）</div>
                  <Input
                    mode="password"
                    className="ai-channel-mono"
                    placeholder={form.channelId ? '已配置，留空表示不修改' : '请输入'}
                    value={form.extraKey}
                    onChange={(v) => setForm({ ...form, extraKey: v })}
                  />
                </div>
              )}
            </div>
          </div>
          {form.kind === 2 && (
            <div className="ai-channel-section">
              <div className="ai-channel-section-head">
                <div className="ai-channel-section-title">模型列表</div>
                <Button
                  size="small"
                  onClick={() =>
                    setForm({
                      ...form,
                      models: [
                        ...form.models,
                        {
                          modelCode: '',
                          displayName: '',
                          capabilities: 'chat',
                          contextTokens: 0,
                          isEnabled: true,
                        },
                      ],
                    })
                  }
                >
                  + 添加模型
                </Button>
              </div>
              {form.models.length === 0 && (
                <div className="ai-channel-empty">暂无模型，点击右上角「+ 添加模型」添加。</div>
              )}
              {form.models.map((m, i) => (
                <div className="ai-model-card" key={i}>
                  <div className="ai-channel-field ai-model-card-code">
                    <div className="ai-channel-label">模型编码</div>
                    <Input
                      className="ai-channel-mono"
                      placeholder="如 deepseek-chat"
                      value={m.modelCode}
                      onChange={(v) => {
                        const models = [...form.models];
                        models[i] = { ...m, modelCode: v };
                        setForm({ ...form, models });
                      }}
                    />
                  </div>
                  <div className="ai-channel-field ai-model-card-name">
                    <div className="ai-channel-label">显示名</div>
                    <Input
                      placeholder="如 DeepSeek Chat"
                      value={m.displayName}
                      onChange={(v) => {
                        const models = [...form.models];
                        models[i] = { ...m, displayName: v };
                        setForm({ ...form, models });
                      }}
                    />
                  </div>
                  <div className="ai-channel-field ai-model-card-cap">
                    <div className="ai-channel-label">能力</div>
                    <Select
                      value={m.capabilities}
                      onChange={(v) => {
                        const models = [...form.models];
                        models[i] = { ...m, capabilities: String(v) };
                        setForm({ ...form, models });
                      }}
                      style={{ width: '100%' }}
                    >
                      <Select.Option value="chat">对话</Select.Option>
                      <Select.Option value="chat,vision">对话+视觉</Select.Option>
                      <Select.Option value="image_gen">图片生成</Select.Option>
                      <Select.Option value="chat,vision,image_gen">对话+视觉+生图</Select.Option>
                    </Select>
                  </div>
                  <div className="ai-model-card-actions">
                    <Switch
                      checked={m.isEnabled}
                      onChange={(checked) => {
                        const models = [...form.models];
                        models[i] = { ...m, isEnabled: checked };
                        setForm({ ...form, models });
                      }}
                    />
                    <span className="ai-model-card-state">{m.isEnabled ? '启用' : '停用'}</span>
                    <Button
                      theme="borderless"
                      type="danger"
                      size="small"
                      onClick={() =>
                        setForm({ ...form, models: form.models.filter((_, j) => j !== i) })
                      }
                    >
                      删除
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
          {form.kind === 1 && (
            <div className="ai-channel-note">
              官方渠道的模型与计费由平台在「官方渠道管理」（P3）中维护；此处仅登记渠道本身。
            </div>
          )}
        </div>
      </Modal>
    </div>
  );
}
