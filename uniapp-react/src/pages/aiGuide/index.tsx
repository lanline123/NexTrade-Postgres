import { useEffect, useRef, useState } from "react";
import { Image, Input, ScrollView, Text, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import FormatPrice from "@/components/format-price/FormatPrice";
import {
    createGuideConversation,
    getGuideConversations,
    getGuideMessages,
    stopGuideChat,
    streamGuideChat,
    GUIDE_LOGIN_EXPIRED_MSG,
    type GuideCard,
    type GuideMessage
} from "@/api/guide";
import { useUserStore } from "@/store/user";
import { handleLogin } from "@/utils/handleLogin";
import { priceGateCopy } from "@/utils/inquiry";
import { t } from "@/utils/i18n";

import "./index.scss";

/**
 * AI 导购（我方新增，受限 MVP）：独立入口页，非 IM 会话。
 * - 仅登录买家可用（未登录给登录引导，后端接口本身也在 require_auth 组）；
 *   登录态被后端拒绝（401）时同样切到引导态并带「去登录」入口，不只留一句提示；
 * - 流式回复（SSE），工具产出的商品卡随消息渲染，价格走既有门控口径
 *   （卡数据已由后端按当前买家身份脱敏：priceVisible=false 时显示门控文案，
 *   可见时用 FormatPrice 渲染，与全站列表/详情同一口径）；
 * - 不做图片搜索/比价（评估定版范围外）。
 */

interface DisplayMsg {
    key: string;
    /** 1 买家 / 2 助手 / 3 商品卡组 */
    role: 1 | 2 | 3;
    content: string;
    cards?: GuideCard[];
    pending?: boolean;
}

const SAMPLES = ["帮我推荐一款电池", "18650 电池多少钱？", "我的订单到哪了？"];

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
            if (cards.length > 0) out.push({ key: `h${r.msgId}`, role: 3, content: "", cards });
        }
    }
    return out;
}

function GuideProductCard({ card }: { card: GuideCard }) {
    const gated = card.priceVisible === false;
    const gate = priceGateCopy(card.priceGate);
    return (
        <View
            className="guide-card"
            onClick={() => Taro.navigateTo({ url: `/pages/product/index?id=${card.productId}` }).catch(() => {})}
        >
            <Image className="guide-card__img" src={card.picThumb || card.picUrl || ""} mode="aspectFill" />
            <View className="guide-card__name">{card.productName}</View>
            {gated ? (
                <Text className="guide-card__gate">{gate.text}</Text>
            ) : (
                <View className="guide-card__price">
                    <FormatPrice priceData={card.productPrice} />
                </View>
            )}
        </View>
    );
}

export default function AiGuide() {
    const token = useUserStore((s) => s.token);
    // 本地有令牌但被后端拒绝（401）：与未登录同形态切引导态，文案用过期提示
    const [authExpired, setAuthExpired] = useState(false);
    const [convId, setConvId] = useState<number | null>(null);
    const [messages, setMessages] = useState<DisplayMsg[]>([]);
    const [input, setInput] = useState("");
    const [sending, setSending] = useState(false);
    const [scrollInto, setScrollInto] = useState("");
    const streamRef = useRef<{ abort: () => void } | null>(null);
    const currentAssistantKey = useRef<string>("");
    const convIdRef = useRef<number | null>(null);

    const setConv = (id: number | null) => {
        convIdRef.current = id;
        setConvId(id);
    };

    // 进页续接最近会话（无则等首条消息由后端建会话）
    useEffect(() => {
        // 令牌变化（重新登录/退出）时复位过期态，避免引导态残留
        setAuthExpired(false);
        if (!token) return;
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
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [token]);

    useEffect(() => {
        if (messages.length > 0) {
            setScrollInto(`anchor-${messages[messages.length - 1].key}`);
        }
    }, [messages]);

    const patchAssistant = (fn: (m: DisplayMsg) => DisplayMsg) => {
        const key = currentAssistantKey.current;
        setMessages((prev) => prev.map((m) => (m.key === key ? fn(m) : m)));
    };

    const send = (raw?: string) => {
        const content = (raw ?? input).trim();
        if (!content || sending) return;
        setInput("");
        const userMsg: DisplayMsg = { key: nextKey(), role: 1, content };
        const assistantMsg: DisplayMsg = { key: nextKey(), role: 2, content: "", pending: true };
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
                    if (status === "done" && cards.length > 0) {
                        const cardMsg: DisplayMsg = { key: nextKey(), role: 3, content: "", cards };
                        setMessages((prev) => [...prev, cardMsg]);
                    }
                },
                onError: (message) => {
                    patchAssistant((m) => (m.content ? m : { ...m, content: message }));
                    Taro.showToast({ title: message, icon: "none" }).catch(() => {});
                    // 令牌被拒：切到带「去登录」入口的引导态（与未登录同形态）
                    if (message === GUIDE_LOGIN_EXPIRED_MSG) setAuthExpired(true);
                },
                onDone: () => {
                    patchAssistant((m) => ({ ...m, pending: false }));
                    setSending(false);
                }
            }
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
            Taro.showToast({ title: t("新建会话失败"), icon: "none" }).catch(() => {});
        }
    };

    if (!token || authExpired) {
        return (
            <View className="guide-login">
                <Text className="guide-login__title">{t("AI 导购")}</Text>
                <Text className="guide-login__desc">
                    {t(authExpired ? GUIDE_LOGIN_EXPIRED_MSG : "登录后可使用导购：帮你挑商品、查订单")}
                </Text>
                <View className="guide-login__btn" onClick={handleLogin}>
                    {t("去登录")}
                </View>
            </View>
        );
    }

    return (
        <View className="guide-page">
            <View className="guide-toolbar">
                <Text className="guide-toolbar__hint">{t("平台导购 · 价格以商品页为准")}</Text>
                <View className="guide-toolbar__new" onClick={newChat}>
                    {t("新会话")}
                </View>
            </View>
            <ScrollView className="guide-scroll" scrollY scrollIntoView={scrollInto} scrollWithAnimation>
                {messages.length === 0 && (
                    <View className="guide-empty">
                        <Text className="guide-empty__title">{t("你好，我是 AI 导购")}</Text>
                        <Text className="guide-empty__desc">{t("说说你想买什么，或问我价格、订单进度")}</Text>
                        <View className="guide-samples">
                            {SAMPLES.map((s) => (
                                <View key={s} className="guide-sample" onClick={() => send(s)}>
                                    {t(s)}
                                </View>
                            ))}
                        </View>
                    </View>
                )}
                {messages.map((m) => (
                    <View key={m.key} id={`anchor-${m.key}`}>
                        {m.role === 1 && (
                            <View className="guide-row guide-row--user">
                                <View className="guide-bubble guide-bubble--user">
                                    <Text>{m.content}</Text>
                                </View>
                            </View>
                        )}
                        {m.role === 2 && (
                            <View className="guide-row guide-row--assistant">
                                <View className="guide-bubble guide-bubble--assistant">
                                    {m.content ? (
                                        <Text>{m.content}</Text>
                                    ) : (
                                        <Text className="guide-thinking">{t("正在为你查找…")}</Text>
                                    )}
                                </View>
                            </View>
                        )}
                        {m.role === 3 && m.cards && (
                            <ScrollView className="guide-cards" scrollX>
                                <View className="guide-cards__inner">
                                    {m.cards.map((c) => (
                                        <GuideProductCard key={c.productId} card={c} />
                                    ))}
                                </View>
                            </ScrollView>
                        )}
                    </View>
                ))}
            </ScrollView>
            <View className="guide-inputbar">
                <Input
                    className="guide-input"
                    value={input}
                    placeholder={t("想买什么？问我")}
                    placeholderStyle="color:#bbb"
                    confirmType="send"
                    onInput={(e) => setInput(e.detail.value)}
                    onConfirm={() => send()}
                />
                {sending ? (
                    <View className="guide-send guide-send--stop" onClick={stop}>
                        {t("停止")}
                    </View>
                ) : (
                    <View className="guide-send" onClick={() => send()}>
                        {t("发送")}
                    </View>
                )}
            </View>
        </View>
    );
}
