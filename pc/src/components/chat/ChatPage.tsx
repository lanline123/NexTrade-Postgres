'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Button, Empty, Input, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { useTranslation } from 'react-i18next';
import {
    fetchConversations,
    fetchMessages,
    ImSocket,
    markConversationRead,
    resolveSocketUrl,
    sendImMessage,
    type ImConversation,
    type ImMessage,
} from '@/lib/im';

/**
 * 在线客服（IM）。
 * Vue 基准：pages/chat/index.vue（816 行）+ components 里 chat/src/* 的子组件。
 *
 * 这里保留核心链路：会话列表 → 消息分页 → 发送文本 → 已读回执 → WS 实时收消息。
 * 相对 Vue 的取舍（都是刻意不做，不是遗漏）：
 * - 不发图片/视频/商品卡/订单卡：Vue 侧要走上传与商品选择器，这里只在**展示**上兼容这些类型；
 * - 不做表情面板与语音；
 * - WS 连不上时（部署没给 nginx 配 `/ws`）降级为 8s 轮询，聊天仍可用。
 */
const POLL_INTERVAL = 8000;
const PAGE_SIZE = 20;

export default function ChatPage() {
    const { t } = useTranslation();
    const [needLogin, setNeedLogin] = useState(false);
    const [conversations, setConversations] = useState<ImConversation[]>([]);
    const [active, setActive] = useState<ImConversation | null>(null);
    const [messages, setMessages] = useState<ImMessage[]>([]);
    const [loadingList, setLoadingList] = useState(true);
    const [loadingMsg, setLoadingMsg] = useState(false);
    const [draft, setDraft] = useState('');
    const [sending, setSending] = useState(false);
    const [connected, setConnected] = useState(false);
    const [hasMore, setHasMore] = useState(true);
    const bottomRef = useRef<HTMLDivElement | null>(null);
    // WS 回调里要用「当前会话」，但连接不能因为切会话重建，所以用 ref 取最新值
    const activeRef = useRef<ImConversation | null>(null);

    const scrollToBottom = useCallback(() => {
        bottomRef.current?.scrollIntoView({ block: 'end' });
    }, []);

    const loadConversations = useCallback(async () => {
        try {
            const result = await fetchConversations(1, 30);
            const records = result.records ?? [];
            setConversations(records);
            setActive((prev) => prev ?? records[0] ?? null);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setLoadingList(false);
        }
    }, []);

    useEffect(() => {
        activeRef.current = active;
    }, [active]);

    useEffect(() => {
        const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
        if (!token) {
            setNeedLogin(true);
            setLoadingList(false);
            return;
        }
        void loadConversations();
    }, [loadConversations]);

    /** 打开会话：拉最近一页消息（desc → 反转为正序显示）并上报已读。 */
    const openConversation = useCallback(
        async (conversation: ImConversation) => {
            setActive(conversation);
            setMessages([]);
            setHasMore(true);
            setLoadingMsg(true);
            try {
                const result = await fetchMessages({
                    shopId: conversation.shopId ?? 0,
                    conversationId: conversation.conversationId,
                    firstId: -1,
                    sortOrder: 'desc',
                });
                const records = [...(result.records ?? [])].reverse();
                setMessages(records);
                setHasMore(records.length >= PAGE_SIZE);
                if (conversation.lastServantId) {
                    void markConversationRead({ shopId: conversation.shopId ?? 0, servantId: conversation.lastServantId }).catch(
                        () => null,
                    );
                }
                setTimeout(scrollToBottom, 30);
            } catch (e) {
                Toast.error((e as Error).message);
            } finally {
                setLoadingMsg(false);
            }
        },
        [scrollToBottom],
    );

    /** 向上加载更多：firstId 取当前最早一条的 id。 */
    const loadMore = async () => {
        if (!active || !hasMore || loadingMsg) return;
        const first = messages.find((item) => !item.pending);
        if (!first || typeof first.id !== 'number') return;
        setLoadingMsg(true);
        try {
            const result = await fetchMessages({
                shopId: active.shopId ?? 0,
                conversationId: active.conversationId,
                firstId: first.id,
                sortOrder: 'desc',
            });
            const older = [...(result.records ?? [])].reverse();
            setMessages((prev) => [...older, ...prev]);
            setHasMore(older.length >= PAGE_SIZE);
        } catch (e) {
            Toast.error((e as Error).message);
        } finally {
            setLoadingMsg(false);
        }
    };

    // WS 连接（token 变了就重连）
    useEffect(() => {
        const token = localStorage.getItem('accessToken') || localStorage.getItem('token');
        if (!token) return;
        const socket = new ImSocket(`${resolveSocketUrl()}?token=${token}&platform=pc`, {
            onStatus: setConnected,
            onMessage: (incoming) => {
                const current = activeRef.current;
                if (!current) return;
                setMessages((prev) => {
                    const fresh = incoming.filter((msg) => {
                        if (prev.some((item) => String(item.id) === String(msg.id))) return false;
                        // 只接受当前会话的消息
                        return msg.shopId === (current.shopId ?? 0);
                    });
                    return fresh.length ? [...prev, ...fresh] : prev;
                });
                setTimeout(scrollToBottom, 30);
                void loadConversations();
            },
            onRead: () => {
                setMessages((prev) => prev.map((item) => (item.type === 2 ? { ...item, isRead: 1 } : item)));
            },
        });
        socket.connect();
        return () => {
            socket.close();
        };
        // active 变化不重建连接，只影响消息过滤
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [loadConversations]);

    // WS 未连上时轮询兜底
    useEffect(() => {
        if (connected || !active) return;
        const timer = setInterval(() => {
            void (async () => {
                try {
                    const result = await fetchMessages({
                        shopId: active.shopId ?? 0,
                        conversationId: active.conversationId,
                        firstId: -1,
                        sortOrder: 'desc',
                    });
                    const records = [...(result.records ?? [])].reverse();
                    setMessages((prev) => {
                        if (records.length > prev.length) return records;
                        return prev;
                    });
                    void loadConversations();
                } catch {
                    // 轮询失败静默，下一轮再试
                }
            })();
        }, POLL_INTERVAL);
        return () => clearInterval(timer);
    }, [connected, active, loadConversations]);

    const send = async () => {
        const text = draft.trim();
        if (!text || !active) return;
        const localKey = `local-${Date.now()}`;
        const optimistic: ImMessage = {
            id: localKey,
            type: 2,
            messageType: 'text',
            content: { messageType: 'text', content: text },
            sendTime: new Date().toISOString(),
            pending: true,
        };
        setMessages((prev) => [...prev, optimistic]);
        setDraft('');
        setSending(true);
        setTimeout(scrollToBottom, 20);
        try {
            const sent = await sendImMessage({
                content: { messageType: 'text', content: text },
                shopId: active.shopId ?? 0,
                conversationId: active.conversationId,
            });
            setMessages((prev) =>
                prev.map((item) =>
                    item.id === localKey && sent?.id !== undefined ? { ...sent, pending: false } : item,
                ),
            );
            void loadConversations();
        } catch (e) {
            setMessages((prev) => prev.map((item) => (item.id === localKey ? { ...item, pending: false, failed: true } : item)));
            Toast.error((e as Error).message);
        } finally {
            setSending(false);
        }
    };

    const titleOf = useCallback(
        (conversation: ImConversation) =>
            conversation.shop?.shopTitle ||
            conversation.user?.nickname ||
            conversation.user?.username ||
            t('在线客服'),
        [t],
    );

    const activeTitle = useMemo(() => (active ? titleOf(active) : t('在线客服')), [active, titleOf, t]);

    if (needLogin) {
        return <Empty title={t('请先登录')} description={<a href="/member/login?returnUrl=%2Fchat">{t('去登录')}</a>} />;
    }

    return (
        <div className="pc-chat">
            <aside className="pc-chat-list">
                <div className="pc-chat-list-head">
                    <Typography.Title heading={5}>{t('消息')}</Typography.Title>
                    <span className={`pc-chat-status${connected ? ' is-on' : ''}`}>
                        {connected ? t('实时连接') : t('轮询模式')}
                    </span>
                </div>
                {loadingList ? (
                    <div className="pc-loading">{t('加载中')}</div>
                ) : conversations.length === 0 ? (
                    <Empty title={t('暂无会话')} />
                ) : (
                    conversations.map((conversation) => {
                        const last = conversation.lastMessage?.[0];
                        const unread = Number(conversation.unreadMessageCount ?? 0);
                        return (
                            <div
                                key={conversation.conversationId}
                                className={`pc-chat-item${active?.conversationId === conversation.conversationId ? ' is-active' : ''}`}
                                onClick={() => void openConversation(conversation)}
                            >
                                <img
                                    className="pc-chat-avatar"
                                    src={conversation.user?.avatar || '/assets/avatar/default.gif'}
                                    alt=""
                                />
                                <div className="pc-chat-item-main">
                                    <div className="pc-chat-item-top">
                                        <span className="pc-ellipsis">{titleOf(conversation)}</span>
                                        <span className="pc-muted pc-chat-time">{conversation.lastUpdateTime ?? ''}</span>
                                    </div>
                                    <div className="pc-chat-item-bottom">
                                        <span className="pc-ellipsis pc-muted">
                                            {last?.messageType === 'text'
                                                ? last.content?.content
                                                : last?.messageTypeText || t('暂无消息')}
                                        </span>
                                        {unread > 0 ? (
                                            <Tag size="small" color="red">
                                                {unread}
                                            </Tag>
                                        ) : null}
                                    </div>
                                </div>
                            </div>
                        );
                    })
                )}
            </aside>

            <section className="pc-chat-main">
                {active ? (
                    <>
                        <div className="pc-chat-main-head">
                            <span>{activeTitle}</span>
                        </div>

                        <div className="pc-chat-messages">
                            {hasMore ? (
                                <div className="pc-chat-more">
                                    <Button size="small" theme="borderless" loading={loadingMsg} onClick={() => void loadMore()}>
                                        {t('加载更多')}
                                    </Button>
                                </div>
                            ) : null}

                            {loadingMsg && messages.length === 0 ? (
                                <div className="pc-loading">{t('加载中')}</div>
                            ) : (
                                messages.map((message) => {
                                    const mine = message.type === 2;
                                    return (
                                        <div
                                            key={String(message.id)}
                                            className={`pc-chat-msg${mine ? ' is-mine' : ''}${message.failed ? ' is-failed' : ''}`}
                                        >
                                            {!mine ? (
                                                <img
                                                    className="pc-chat-msg-avatar"
                                                    src={message.user?.avatar || '/assets/avatar/default.gif'}
                                                    alt=""
                                                />
                                            ) : null}
                                            <div className="pc-chat-bubble">
                                                {message.messageType === 'image' && message.content?.pic ? (
                                                    <img className="pc-chat-image" src={message.content.pic} alt="" />
                                                ) : message.messageType === 'product' && message.content?.product ? (
                                                    <a
                                                        className="pc-chat-card"
                                                        href={`/product/${message.content.product.productSn ?? message.content.product.productId ?? ''}`}
                                                    >
                                                        <img src={message.content.product.picUrl} alt="" />
                                                        <div>
                                                            <div className="pc-ellipsis">{message.content.product.productName}</div>
                                                            <div className="pc-promo-price">
                                                                ¥{message.content.product.productPrice}
                                                            </div>
                                                        </div>
                                                    </a>
                                                ) : message.messageType === 'order' && message.content?.order ? (
                                                    <a
                                                        className="pc-chat-card"
                                                        href={`/member/order/info?id=${message.content.order.orderId ?? ''}`}
                                                    >
                                                        <img src={message.content.order.picUrl} alt="" />
                                                        <div>
                                                            <div className="pc-ellipsis">
                                                                {message.content.order.orderSn} · {message.content.order.orderStatusName}
                                                            </div>
                                                            <div className="pc-muted">
                                                                {message.content.order.productName} ×
                                                                {message.content.order.quantity ?? 1}
                                                            </div>
                                                        </div>
                                                    </a>
                                                ) : (
                                                    message.content?.content ?? ''
                                                )}
                                                {message.pending ? <span className="pc-chat-flag">{t('发送中')}</span> : null}
                                                {message.failed ? <span className="pc-chat-flag is-err">{t('发送失败')}</span> : null}
                                            </div>
                                            <div className="pc-chat-meta">{message.sendTime ?? ''}</div>
                                        </div>
                                    );
                                })
                            )}
                            <div ref={bottomRef} />
                        </div>

                        <div className="pc-chat-input">
                            <Input
                                value={draft}
                                placeholder={t('输入消息，回车发送')}
                                onChange={(v) => setDraft(String(v))}
                                onEnterPress={() => void send()}
                            />
                            <Button theme="solid" type="primary" loading={sending} onClick={() => void send()}>
                                {t('发送')}
                            </Button>
                        </div>
                    </>
                ) : (
                    <Empty title={t('选择左侧会话开始聊天')} />
                )}
            </section>
        </div>
    );
}
