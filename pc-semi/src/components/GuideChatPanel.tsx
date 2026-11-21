'use client';

/**
 * AI 导购聊天面板（我方新增）—— PC 端入口收编进全站右下角悬浮工具栏
 * GlobalSideTools 的第一位（用户 2026-11-21 指令），点击打开本抽屉。
 *
 * 与 H5 导购页（uniapp-react/src/pages/aiGuide）同一套后端接口与口径：
 * - 仅登录买家可用（未登录给登录引导，后端接口本身也在 require_auth 组）；
 * - 打开时续接最近会话（无则首条消息由后端建会话），新会话按钮手动重开；
 * - 流式回复（SSE），工具产出的商品卡随消息渲染，价格走既有门控口径
 *   （卡数据已由后端按当前买家身份脱敏：priceVisible=false 时渲染
 *   gateCopyForPayload 文案，与全站列表/详情同一口径；可见时用
 *   formatDisplayPrice 渲染，与 PC 商品卡同一展示口径）；
 * - 商品卡点击跳 /product/{id}（新标签，与 PC 商品卡口径一致）。
 *
 * 弹层形态：Semi SideSheet 右侧抽屉（PC 站既有弹层体系 @douyinfe/semi-ui，
 * 与 InquiryFormModal 的 Modal 同系），zIndex 高于悬浮工具栏（1100）。
 */
import { useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import { Button, Input, SideSheet, Toast } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import {
  createGuideConversation,
  getGuideConversations,
  getGuideMessages,
  stopGuideChat,
  streamGuideChat,
  type GuideCard,
  type GuideMessage,
} from '@/lib/guide-api';
import { gateCopyForPayload, hasAccessToken } from '@/lib/price-gate';
import { formatDisplayPrice } from '@/lib/currency';
import './guide-chat.scss';

interface DisplayMsg {
  key: string;
  /** 1 买家 / 2 助手 / 3 商品卡组 */
  role: 1 | 2 | 3;
  content: string;
  cards?: GuideCard[];
  pending?: boolean;
}

const SAMPLES = ['帮我推荐一款电池', '18650 电池多少钱？', '我的订单到哪了？'];

let seq = 0;
const nextKey = () => `m${Date.now()}-${seq++}`;

function toDisplay(rows: GuideMessage[]): DisplayMsg[] {
  const out: DisplayMsg[] = [];
  for (const r of rows) {
    if (r.role === 1) {
      out.push({ key: `h${r.msgId}`, role: 1, content: r.content });
    } else if (r.role === 2) {
      // 纯工具调用轮（无文本）不单独渲染，其商品卡由随后的工具行呈现
      if (r.content) out.push({ key: `h${r.msgId}`, role: 2, content: r.content });
    } else if (r.role === 3) {
      const cards = r.toolCalls?.cards || [];
      if (cards.length > 0) out.push({ key: `h${r.msgId}`, role: 3, content: '', cards });
    }
  }
  return out;
}

function GuideProductCard({ card }: { card: GuideCard }) {
  const { t } = useTranslation();
  const gated = card.priceVisible === false;
  const gate = gateCopyForPayload(card.priceGate);
  return (
    <div className="guide-card">
      <Link className="guide-card__main" href={`/product/${card.productId}`} target="_blank">
        <img
          className="guide-card__img"
          src={card.picThumb || card.picUrl || ''}
          alt={card.productName}
        />
        <div className="guide-card__name">{card.productName}</div>
      </Link>
      {gated ? (
        <Link className="guide-card__gate" href={gate.href} target="_blank">
          {t(gate.text)}
        </Link>
      ) : (
        <div className="guide-card__price">{formatDisplayPrice(card.productPrice)}</div>
      )}
    </div>
  );
}

export interface GuideChatPanelProps {
  open: boolean;
  onClose: () => void;
}

export default function GuideChatPanel({ open, onClose }: GuideChatPanelProps) {
  const { t } = useTranslation();
  const [loggedIn, setLoggedIn] = useState(false);
  const [, setConvId] = useState<number | null>(null);
  const [messages, setMessages] = useState<DisplayMsg[]>([]);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const resumedRef = useRef(false);
  const streamRef = useRef<{ abort: () => void } | null>(null);
  const currentAssistantKey = useRef('');
  const convIdRef = useRef<number | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const setConv = (id: number | null) => {
    convIdRef.current = id;
    setConvId(id);
  };

  // 打开时刷新登录态；首次打开续接最近会话（与 H5 导购同口径）
  useEffect(() => {
    if (!open) return;
    const has = hasAccessToken();
    setLoggedIn(has);
    if (!has || resumedRef.current) return;
    resumedRef.current = true;
    let cancelled = false;
    (async () => {
      try {
        const { list } = await getGuideConversations();
        if (cancelled || !list || list.length === 0) return;
        const latest = list[0];
        setConv(latest.convId);
        const res = await getGuideMessages(latest.convId);
        if (!cancelled && res?.list) setMessages(toDisplay(res.list));
      } catch {
        // 续接失败不阻塞：按新会话开始
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open ]);

  // 新消息自动滚到底
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages, open]);

  const patchAssistant = (fn: (m: DisplayMsg) => DisplayMsg) => {
    const key = currentAssistantKey.current;
    setMessages((prev) => prev.map((m) => (m.key === key ? fn(m) : m)));
  };

  const send = (raw?: string) => {
    const content = (raw ?? input).trim();
    if (!content || sending) return;
    setInput('');
    const userMsg: DisplayMsg = { key: nextKey(), role: 1, content };
    const assistantMsg: DisplayMsg = { key: nextKey(), role: 2, content: '', pending: true };
    currentAssistantKey.current = assistantMsg.key;
    setMessages((prev) => [...prev, userMsg, assistantMsg]);
    setSending(true);
    streamRef.current = streamGuideChat(
      { convId: convIdRef.current ?? undefined, content },
      {
        onAssistant: (_msgId, newConvId) => {
          if (newConvId) setConv(newConvId);
        },
        onDelta: (delta) => {
          patchAssistant((m) => ({ ...m, content: m.content + delta }));
        },
        onTool: (_name, status, cards) => {
          if (status === 'done' && cards.length > 0) {
            const cardMsg: DisplayMsg = { key: nextKey(), role: 3, content: '', cards };
            setMessages((prev) => [...prev, cardMsg]);
          }
        },
        onError: (message) => {
          const text = t(message);
          patchAssistant((m) => (m.content ? m : { ...m, content: text }));
          Toast.error(text);
        },
        onDone: () => {
          patchAssistant((m) => ({ ...m, pending: false }));
          setSending(false);
        },
      },
    );
  };

  const stop = async () => {
    streamRef.current?.abort();
    if (convIdRef.current) {
      try {
        await stopGuideChat(convIdRef.current);
      } catch {
        // 停止请求失败不阻塞本地收口
      }
    }
    patchAssistant((m) => ({ ...m, pending: false }));
    setSending(false);
  };

  const newChat = async () => {
    if (sending) return;
    try {
      const { convId: id } = await createGuideConversation();
      setConv(id);
      setMessages([]);
    } catch {
      Toast.error(t('新建会话失败'));
    }
  };

  return (
    <SideSheet
      visible={open}
      onCancel={onClose}
      width={420}
      placement="right"
      title={t('AI 导购')}
      closable
      zIndex={1200}
      className="guide-sheet"
      bodyStyle={{ padding: 0 }}
    >
      {!loggedIn ? (
        <div className="guide-login">
          <div className="guide-login__title">{t('AI 导购')}</div>
          <div className="guide-login__desc">{t('登录后可使用导购：帮你挑商品、查订单')}</div>
          <Link href="/login">
            <Button theme="solid" type="primary">
              {t('去登录')}
            </Button>
          </Link>
        </div>
      ) : (
        <div className="guide-chat">
          <div className="guide-toolbar">
            <span className="guide-toolbar__hint">{t('平台导购 · 价格以商品页为准')}</span>
            <Button theme="borderless" size="small" onClick={newChat} disabled={sending}>
              {t('新会话')}
            </Button>
          </div>
          <div className="guide-scroll" ref={scrollRef}>
            {messages.length === 0 ? (
              <div className="guide-empty">
                <div className="guide-empty__title">{t('你好，我是 AI 导购')}</div>
                <div className="guide-empty__desc">
                  {t('说说你想买什么，或问我价格、订单进度')}
                </div>
                <div className="guide-samples">
                  {SAMPLES.map((s) => (
                    <button
                      key={s}
                      type="button"
                      className="guide-sample"
                      onClick={() => send(s)}
                    >
                      {t(s)}
                    </button>
                  ))}
                </div>
              </div>
            ) : null}
            {messages.map((m) =>
              m.role === 3 && m.cards ? (
                <div key={m.key} className="guide-cards">
                  {m.cards.map((c) => (
                    <GuideProductCard key={c.productId} card={c} />
                  ))}
                </div>
              ) : (
                <div
                  key={m.key}
                  className={`guide-row guide-row--${m.role === 1 ? 'user' : 'assistant'}`}
                >
                  <div className="guide-bubble">
                    {m.content ? (
                      m.content
                    ) : (
                      <span className="guide-thinking">{t('正在为你查找…')}</span>
                    )}
                  </div>
                </div>
              ),
            )}
          </div>
          <div className="guide-inputbar">
            <Input
              value={input}
              placeholder={t('想买什么？问我')}
              onChange={(value) => setInput(value)}
              onEnterPress={() => send()}
            />
            {sending ? (
              <Button theme="solid" type="danger" onClick={stop}>
                {t('停止')}
              </Button>
            ) : (
              <Button theme="solid" type="primary" onClick={() => send()}>
                {t('发送')}
              </Button>
            )}
          </div>
        </div>
      )}
    </SideSheet>
  );
}
