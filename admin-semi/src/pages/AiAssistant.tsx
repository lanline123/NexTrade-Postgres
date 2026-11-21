/**
 * AI 助手对话页（我方新增）：类 DeepSeek 三段式。
 * 左：会话列表；中：对话流（流式输出、工具卡、写动作确认卡）；顶：渠道/模型选择器。
 * 服务端是状态源：流式增量只做临时渲染，工具/确认/结束事件后静默回拉消息列表对齐
 * （不切整区加载态，流式气泡保留到对齐数据落地才收起，结论原地完成、不整页重载感）。
 * 助手文本经共享组件 MarkdownText 按 Markdown 基础子集渲染（加粗/标题/列表/代码/链接）。
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Button,
  Checkbox,
  Empty,
  Select,
  Spin,
  Tag,
  TextArea,
  Toast,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost, getContextHeaders, getToken } from '../api';
import MarkdownText from '../components/MarkdownText';

interface AiModel {
  modelId: number;
  modelCode: string;
  displayName: string;
  capabilities: string;
  isEnabled: boolean;
}

interface AiChannel {
  channelId: number;
  name: string;
  kind: number;
  baseUrl: string;
  isEnabled: boolean;
  isDefault: boolean;
  keyMask: string;
  hasKey: boolean;
  official: boolean;
  models: AiModel[];
}

interface Conversation {
  convId: number;
  title: string;
  channelId: number;
  modelCode: string;
  adminId: number;
  updateTime: number;
}

interface ToolCallInfo {
  callId?: string;
  name?: string;
  args?: Record<string, unknown>;
}

interface Message {
  msgId: number;
  role: number; // 1 user / 2 assistant / 3 tool
  content: string;
  toolCalls?: ToolCallInfo | ToolCallInfo[] | null;
  toolName: string;
  toolStatus: number; // 1 待确认 / 2 已执行 / 3 已拒绝 / 4 失败
  promptTokens: number;
  completionTokens: number;
  createTime: number;
}

const TOOL_TITLES: Record<string, string> = {
  product_search: '商品搜索',
  product_get: '商品详情',
  product_list_categories: '分类列表',
  product_list_brands: '品牌列表',
  product_list_skus: 'SKU 列表',
  product_update_fields: '修改商品信息',
  product_update_price: '修改商品价格',
  product_update_stock: '修改商品库存',
  product_publish: '商品上架',
  product_unpublish: '商品下架',
  product_batch_preview: '批量操作预览',
  product_batch_execute: '批量操作',
  product_import: '批量导入商品',
  product_import_validate: '导入校验',
  product_copy: '复制商品',
  product_analysis: '商品分析',
  product_context: '商品样本统计',
  product_category_create: '创建商品分类',
  media_upload: '上传素材',
  media_manage: '素材管理',
  decorate_list_pages: '装修页列表',
  decorate_get_page: '装修页详情',
  decorate_save_draft: '保存装修草稿',
  decorate_publish: '发布装修页',
  analytics_sales_summary: '销售汇总',
  analytics_top_products: '热销商品',
  analytics_low_stock: '低库存预警',
  order_search: '订单搜索',
  order_get: '订单详情',
  order_update_remark: '修改订单备注',
  order_deliver: '订单发货',
  member_search: '会员搜索',
  member_get: '会员详情',
  content_article_list: '文章列表',
  content_article_get: '文章详情',
};

async function consumeSse(
  path: string,
  body: unknown,
  onEvent: (event: string, data: Record<string, unknown>) => void,
  signal: AbortSignal,
): Promise<void> {
  const res = await fetch(`/adminapi${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${getToken()}`,
      ...getContextHeaders(),
    },
    body: JSON.stringify(body),
    signal,
  });
  if (!res.ok || !res.body) {
    throw new Error(`请求失败（${res.status}）`);
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buffer.indexOf('\n\n')) >= 0) {
      const raw = buffer.slice(0, idx);
      buffer = buffer.slice(idx + 2);
      let event = 'message';
      let data = '';
      raw.split('\n').forEach((line) => {
        if (line.startsWith('event:')) event = line.slice(6).trim();
        if (line.startsWith('data:')) data += line.slice(5).trim();
      });
      if (data) {
        try {
          onEvent(event, JSON.parse(data) as Record<string, unknown>);
        } catch {
          /* 忽略无法解析的事件 */
        }
      }
    }
  }
}

export default function AiAssistant() {
  const [channels, setChannels] = useState<AiChannel[]>([]);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [selChannel, setSelChannel] = useState<number>(0);
  const [selModel, setSelModel] = useState<string>('');
  const [input, setInput] = useState('');
  const [streaming, setStreaming] = useState(false);
  const [liveText, setLiveText] = useState('');
  const [loadingMsgs, setLoadingMsgs] = useState(false);
  const abortRef = useRef<AbortController | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  // 本会话内免确认集合（按会话存 localStorage，仅普通写在服务端采信，高风险永不免）
  const [skipMap, setSkipMap] = useState<Record<number, string[]>>(() => {
    try {
      return JSON.parse(localStorage.getItem('ai_skip_confirm') || '{}') as Record<number, string[]>;
    } catch {
      return {};
    }
  });

  const chatChannels = channels.filter(
    (c) => c.isEnabled && (c.kind === 2 || c.official),
  );
  const currentChannel = chatChannels.find((c) => c.channelId === selChannel);
  const chatModels = (currentChannel?.models || []).filter(
    (m) => m.isEnabled && m.capabilities.includes('chat'),
  );

  const loadChannels = useCallback(async () => {
    try {
      const res = await apiGet<{ channels: AiChannel[] }>('/ai/channels');
      setChannels(res.channels || []);
    } catch (e) {
      Toast.error({ content: (e as Error).message || '加载渠道失败' });
    }
  }, []);

  const loadConversations = useCallback(async () => {
    try {
      const res = await apiGet<{ conversations: Conversation[] }>('/ai/conversations');
      setConversations(res.conversations || []);
    } catch {
      /* 列表失败不阻塞对话 */
    }
  }, []);

  // silent=true：后台对齐刷新——不切整区加载态，现有消息保持显示，数据落地时原地替换；
  // onAligned 在 setMessages 同一同步段内回调（流式收尾用它收起流式气泡，保证单次渲染完成切换）。
  const loadMessages = useCallback(
    async (convId: number, opts?: { silent?: boolean; onAligned?: () => void }) => {
      if (!opts?.silent) setLoadingMsgs(true);
      try {
        const res = await apiGet<{ conversation: Conversation; messages: Message[] }>(
          `/ai/conversations/${convId}/messages`,
        );
        setMessages(res.messages || []);
        setSelChannel(res.conversation.channelId);
        setSelModel(res.conversation.modelCode);
        opts?.onAligned?.();
      } catch (e) {
        Toast.error({ content: (e as Error).message || '加载会话失败' });
      } finally {
        if (!opts?.silent) setLoadingMsgs(false);
      }
    },
    [],
  );

  useEffect(() => {
    void loadChannels();
    void loadConversations();
  }, [loadChannels, loadConversations]);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, liveText]);

  const pickDefaultChannel = useCallback(() => {
    const usable = channels.filter((c) => c.isEnabled && c.kind === 2);
    const def = usable.find((c) => c.isDefault) || usable[0];
    if (def) {
      setSelChannel(def.channelId);
      const m = def.models.find((x) => x.isEnabled && x.capabilities.includes('chat'));
      setSelModel(m?.modelCode || '');
    }
  }, [channels]);

  useEffect(() => {
    if (activeId === null && selChannel === 0 && channels.length > 0) {
      pickDefaultChannel();
    }
  }, [channels, activeId, selChannel, pickDefaultChannel]);

  const openConversation = (convId: number) => {
    setActiveId(convId);
    setLiveText('');
    void loadMessages(convId);
  };

  const newConversation = () => {
    setActiveId(null);
    setMessages([]);
    setLiveText('');
    pickDefaultChannel();
  };

  const deleteConversation = async (convId: number) => {
    try {
      const res = await fetch(`/adminapi/ai/conversations/${convId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${getToken()}`, ...getContextHeaders() },
      });
      if (!res.ok) throw new Error('删除失败');
      if (activeId === convId) newConversation();
      void loadConversations();
    } catch (e) {
      Toast.error({ content: (e as Error).message || '删除失败' });
    }
  };

  const runStream = useCallback(
    async (path: string, body: Record<string, unknown>, convIdForReload: number | null) => {
      const controller = new AbortController();
      abortRef.current = controller;
      setStreaming(true);
      setLiveText('');
      let errorMsg = '';
      try {
        await consumeSse(
          path,
          body,
          (event, data) => {
            if (event === 'delta') {
              setLiveText((prev) => prev + String(data.content || ''));
            } else if (event === 'error') {
              errorMsg = String(data.message || '生成失败');
            }
          },
          controller.signal,
        );
      } catch (e) {
        if ((e as Error).name !== 'AbortError') {
          errorMsg = (e as Error).message || '连接失败';
        }
      } finally {
        setStreaming(false);
        abortRef.current = null;
        if (errorMsg) Toast.error({ content: errorMsg });
        // 收尾对齐一律走静默刷新：流式气泡保留到对齐数据落地、由 onAligned 与
        // setMessages 同步收起，结论原地完成——不再先清空气泡再整区转圈重拉。
        await loadConversations();
        const target = convIdForReload ?? activeId;
        if (target) {
          await loadMessages(target, { silent: true, onAligned: () => setLiveText('') });
        } else {
          // 新会话：刷新列表后打开最新一条
          const res = await apiGet<{ conversations: Conversation[] }>('/ai/conversations').catch(
            () => null,
          );
          const latest = res?.conversations?.[0];
          if (latest) {
            setActiveId(latest.convId);
            await loadMessages(latest.convId, {
              silent: true,
              onAligned: () => setLiveText(''),
            });
          }
        }
        // 兜底：对齐失败时也收起流式气泡，避免残影（成功路径已由 onAligned 同步收起）
        setLiveText('');
      }
    },
    [activeId, loadConversations, loadMessages],
  );

  const send = async () => {
    const content = input.trim();
    if (!content || streaming) return;
    if (!selChannel) {
      Toast.warning({ content: '请先在「渠道配置」中添加渠道与密钥' });
      return;
    }
    setInput('');
    setMessages((prev) => [
      ...prev,
      {
        msgId: -Date.now(),
        role: 1,
        content,
        toolName: '',
        toolStatus: 0,
        promptTokens: 0,
        completionTokens: 0,
        createTime: Math.floor(Date.now() / 1000),
      },
    ]);
    const skip = activeId ? skipMap[activeId] || [] : [];
    await runStream(
      '/ai/chat/stream',
      {
        convId: activeId,
        content,
        channelId: selChannel,
        modelCode: selModel || undefined,
        skipConfirmTools: skip,
      },
      activeId,
    );
  };

  const stop = async () => {
    abortRef.current?.abort();
    if (activeId) {
      await apiPost('/ai/chat/stop', { convId: activeId }).catch(() => undefined);
    }
  };

  const confirm = async (
    msg: Message,
    decision: 'approve' | 'reject',
    patchedArgs?: Record<string, unknown>,
    skipSameTool?: boolean,
  ) => {
    if (!activeId || streaming) return;
    if (decision === 'approve' && skipSameTool && msg.toolName) {
      const next = {
        ...skipMap,
        [activeId]: Array.from(new Set([...(skipMap[activeId] || []), msg.toolName])),
      };
      setSkipMap(next);
      localStorage.setItem('ai_skip_confirm', JSON.stringify(next));
    }
    await runStream(
      '/ai/chat/confirm',
      {
        msgId: msg.msgId,
        decision,
        patchedArgs,
        skipConfirmTools: activeId ? skipMap[activeId] || [] : [],
      },
      activeId,
    );
  };

  const switchModel = async (channelId: number, modelCode: string) => {
    setSelChannel(channelId);
    setSelModel(modelCode);
    if (activeId) {
      try {
        await apiPost(`/ai/conversations/${activeId}/model`, { channelId, modelCode });
      } catch (e) {
        Toast.error({ content: (e as Error).message || '切换模型失败' });
      }
    }
  };

  return (
    <div style={{ display: 'flex', gap: 12, height: 'calc(100vh - 120px)' }}>
      {/* 左栏：会话列表 */}
      <div
        style={{
          width: 250,
          flexShrink: 0,
          border: '1px solid var(--semi-color-border)',
          borderRadius: 8,
          display: 'flex',
          flexDirection: 'column',
          background: 'var(--semi-color-bg-1)',
        }}
      >
        <div style={{ padding: 12 }}>
          <Button block theme="solid" onClick={newConversation}>
            + 新建会话
          </Button>
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '0 8px 8px' }}>
          {conversations.length === 0 && (
            <Empty title="暂无会话" description="发送第一条消息开始" />
          )}
          {conversations.map((c) => (
            <div
              key={c.convId}
              onClick={() => openConversation(c.convId)}
              style={{
                padding: '8px 10px',
                borderRadius: 6,
                cursor: 'pointer',
                marginBottom: 4,
                background:
                  activeId === c.convId ? 'var(--semi-color-primary-light-default)' : 'transparent',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                gap: 6,
              }}
            >
              <span
                style={{
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                  fontSize: 13,
                }}
              >
                {c.title || '新会话'}
              </span>
              <Button
                size="small"
                type="tertiary"
                onClick={(e) => {
                  e.stopPropagation();
                  void deleteConversation(c.convId);
                }}
              >
                删除
              </Button>
            </div>
          ))}
        </div>
      </div>

      {/* 中栏：对话 */}
      <div
        style={{
          flex: 1,
          border: '1px solid var(--semi-color-border)',
          borderRadius: 8,
          display: 'flex',
          flexDirection: 'column',
          background: 'var(--semi-color-bg-1)',
          minWidth: 0,
        }}
      >
        {/* 顶栏：渠道/模型 + 提示 */}
        <div
          style={{
            padding: '10px 12px',
            borderBottom: '1px solid var(--semi-color-border)',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            flexWrap: 'wrap',
          }}
        >
          <Select
            style={{ width: 200 }}
            placeholder="选择渠道"
            value={selChannel || undefined}
            onChange={(v) => {
              const ch = chatChannels.find((c) => c.channelId === v);
              const m = ch?.models.find((x) => x.isEnabled && x.capabilities.includes('chat'));
              void switchModel(Number(v), m?.modelCode || '');
            }}
          >
            {chatChannels.map((c) => (
              <Select.Option key={c.channelId} value={c.channelId}>
                {c.official ? `官方渠道 · ${c.name}` : c.name}
                {!c.official && !c.hasKey ? '（未配置密钥）' : ''}
              </Select.Option>
            ))}
          </Select>
          <Select
            style={{ width: 220 }}
            placeholder="选择模型"
            value={selModel || undefined}
            onChange={(v) => void switchModel(selChannel, String(v))}
          >
            {chatModels.map((m) => (
              <Select.Option key={m.modelCode} value={m.modelCode}>
                {m.displayName || m.modelCode}
              </Select.Option>
            ))}
          </Select>
          <span style={{ color: 'var(--semi-color-text-2)', fontSize: 12 }}>
            {currentChannel?.official
              ? '官方渠道 · 按积分计费（P3 上线）'
              : '自有密钥 · 不消耗积分'}
            {currentChannel && !currentChannel.official && !currentChannel.hasKey
              ? ' · 该渠道尚未配置密钥'
              : ''}
          </span>
        </div>

        {/* 消息流 */}
        <div ref={scrollRef} style={{ flex: 1, overflowY: 'auto', padding: 16 }}>
          {loadingMsgs ? (
            <Spin />
          ) : messages.length === 0 && !liveText ? (
            <Empty
              title="开始一段对话"
              description="可以让我查销量、看低库存、改价格、上下架、给订单发货等——写操作都会先请你确认"
            />
          ) : (
            messages.map((m) => (
              <MessageRow key={m.msgId} msg={m} onConfirm={confirm} streaming={streaming} />
            ))
          )}
          {liveText && (
            <div style={{ display: 'flex', marginBottom: 12 }}>
              <div
                style={{
                  maxWidth: '78%',
                  background: 'var(--semi-color-fill-0)',
                  borderRadius: 8,
                  padding: '10px 12px',
                  fontSize: 14,
                  lineHeight: 1.6,
                }}
              >
                <MarkdownText text={liveText} />
                {streaming && <span style={{ opacity: 0.5 }}>▍</span>}
              </div>
            </div>
          )}
          {streaming && !liveText && (
            <div style={{ color: 'var(--semi-color-text-2)', fontSize: 13 }}>正在思考…</div>
          )}
        </div>

        {/* 输入区 */}
        <div style={{ padding: 12, borderTop: '1px solid var(--semi-color-border)' }}>
          <TextArea
            value={input}
            onChange={setInput}
            placeholder="输入你的指令，例如：查一下近 7 天的销量 / 把低库存的商品列出来 / 给订单 12345 发货"
            autosize={{ minRows: 2, maxRows: 5 }}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                void send();
              }
            }}
          />
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginTop: 8,
            }}
          >
            <span style={{ color: 'var(--semi-color-text-2)', fontSize: 12 }}>
              回车发送，Shift+回车换行。写操作执行前都会弹出确认卡。
            </span>
            {streaming ? (
              <Button theme="solid" type="danger" onClick={() => void stop()}>
                停止生成
              </Button>
            ) : (
              <Button theme="solid" onClick={() => void send()} disabled={!input.trim()}>
                发送
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function MessageRow({
  msg,
  onConfirm,
  streaming,
}: {
  msg: Message;
  onConfirm: (
    msg: Message,
    decision: 'approve' | 'reject',
    patchedArgs?: Record<string, unknown>,
    skipSameTool?: boolean,
  ) => Promise<void>;
  streaming: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const [skipSame, setSkipSame] = useState(false);
  const [editArgs, setEditArgs] = useState(false);
  const [argsText, setArgsText] = useState('');

  if (msg.role === 1) {
    return (
      <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 12 }}>
        <div
          style={{
            maxWidth: '78%',
            background: 'var(--semi-color-primary)',
            color: '#fff',
            borderRadius: 8,
            padding: '10px 12px',
            whiteSpace: 'pre-wrap',
            fontSize: 14,
            lineHeight: 1.6,
          }}
        >
          {msg.content}
        </div>
      </div>
    );
  }

  if (msg.role === 3) {
    const info = (msg.toolCalls || {}) as ToolCallInfo;
    const args = info.args || {};
    const title = TOOL_TITLES[msg.toolName] || msg.toolName;
    const statusTag =
      msg.toolStatus === 1 ? (
        <Tag color="amber">待确认</Tag>
      ) : msg.toolStatus === 2 ? (
        <Tag color="green">已执行</Tag>
      ) : msg.toolStatus === 3 ? (
        <Tag color="grey">已拒绝</Tag>
      ) : (
        <Tag color="red">失败</Tag>
      );
    return (
      <div
        style={{
          marginBottom: 12,
          border: '1px solid var(--semi-color-border)',
          borderRadius: 8,
          padding: '10px 12px',
          maxWidth: '86%',
          background:
            msg.toolStatus === 1 ? 'var(--semi-color-warning-light-default)' : 'transparent',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
          <strong style={{ fontSize: 13 }}>{title}</strong>
          {statusTag}
          <Button size="small" type="tertiary" onClick={() => setExpanded(!expanded)}>
            {expanded ? '收起' : '详情'}
          </Button>
        </div>
        <div style={{ fontSize: 12, color: 'var(--semi-color-text-2)', marginBottom: 4 }}>
          参数：{JSON.stringify(args)}
        </div>
        {expanded && msg.content && (
          <pre
            style={{
              fontSize: 12,
              background: 'var(--semi-color-fill-0)',
              padding: 8,
              borderRadius: 6,
              overflowX: 'auto',
              maxHeight: 260,
              overflowY: 'auto',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
            }}
          >
            {msg.content}
          </pre>
        )}
        {!expanded && msg.toolStatus !== 1 && msg.content && (
          <div
            style={{
              fontSize: 12,
              color: 'var(--semi-color-text-2)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            结果：{msg.content.slice(0, 160)}
          </div>
        )}
        {msg.toolStatus === 1 && (
          <div style={{ marginTop: 8 }}>
            <div style={{ fontSize: 13, marginBottom: 8 }}>
              AI 请求执行这个写操作，请核对后确认：
            </div>
            {editArgs ? (
              <TextArea
                value={argsText}
                onChange={setArgsText}
                autosize={{ minRows: 3, maxRows: 8 }}
                placeholder="以 JSON 修改参数"
              />
            ) : null}
            <div
              style={{
                display: 'flex',
                gap: 8,
                alignItems: 'center',
                marginTop: 8,
                flexWrap: 'wrap',
              }}
            >
              <Button
                theme="solid"
                disabled={streaming}
                onClick={() => {
                  let patched: Record<string, unknown> | undefined;
                  if (editArgs) {
                    try {
                      patched = JSON.parse(argsText) as Record<string, unknown>;
                    } catch {
                      Toast.error({ content: '参数 JSON 格式不正确' });
                      return;
                    }
                  }
                  void onConfirm(msg, 'approve', patched, skipSame);
                }}
              >
                批准执行
              </Button>
              <Button disabled={streaming} onClick={() => void onConfirm(msg, 'reject')}>
                拒绝
              </Button>
              <Button
                type="tertiary"
                onClick={() => {
                  setEditArgs(!editArgs);
                  setArgsText(JSON.stringify(args, null, 2));
                }}
              >
                {editArgs ? '取消改参' : '修改参数'}
              </Button>
              <Checkbox checked={skipSame} onChange={(e) => setSkipSame(!!e.target.checked)}>
                本会话内同类动作不再询问（高风险动作除外）
              </Checkbox>
            </div>
          </div>
        )}
      </div>
    );
  }

  // role 2：assistant 文本（Markdown 基础子集渲染，与流式气泡同一管线）
  if (!msg.content) return null;
  return (
    <div style={{ display: 'flex', marginBottom: 12 }}>
      <div
        style={{
          maxWidth: '78%',
          background: 'var(--semi-color-fill-0)',
          borderRadius: 8,
          padding: '10px 12px',
          fontSize: 14,
          lineHeight: 1.6,
        }}
      >
        <MarkdownText text={msg.content} />
        {msg.completionTokens > 0 && (
          <div style={{ marginTop: 6, fontSize: 11, color: 'var(--semi-color-text-2)' }}>
            本次消耗 {msg.promptTokens + msg.completionTokens} tokens（自有密钥 · 不计积分）
          </div>
        )}
      </div>
    </div>
  );
}
