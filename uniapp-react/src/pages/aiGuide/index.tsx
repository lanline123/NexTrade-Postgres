import { useEffect, useRef, useState, type ReactNode } from "react";
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

/* ---------- GFM 表格段（导购回复的表格原先随纯文本裸渲染、竖线原样显示错版，
   本批把表格段解析为结构化网格；文本段仍走 <Text> 直出，全量 Markdown 渲染
   仍属既登记的独立立项，不在本批） ---------- */

type GuideAlign = "left" | "center" | "right" | undefined;

type GuideSegment =
    | { type: "text"; text: string }
    | { type: "table"; header: string[]; aligns: GuideAlign[]; rows: string[][] };

/** 拆一行表格为单元格：处理首尾竖线与 \| 转义（与 PC GuideMarkdown 同口径）。 */
function splitTableRow(line: string): string[] {
    let t = line.trim();
    if (t.startsWith("|")) t = t.slice(1);
    if (t.endsWith("|") && !t.endsWith("\\|")) t = t.slice(0, -1);
    const cells: string[] = [];
    let cur = "";
    for (let i = 0; i < t.length; i += 1) {
        if (t[i] === "\\" && t[i + 1] === "|") {
            cur += "|";
            i += 1;
            continue;
        }
        if (t[i] === "|") {
            cells.push(cur.trim());
            cur = "";
            continue;
        }
        cur += t[i];
    }
    cells.push(cur.trim());
    return cells;
}

function parseSepAlign(cell: string): GuideAlign | null {
    const c = cell.trim();
    if (!/^:?-+:?$/.test(c)) return null;
    const left = c.startsWith(":");
    const right = c.endsWith(":");
    if (left && right) return "center";
    if (right) return "right";
    if (left) return "left";
    return undefined;
}

function isTableSepRow(line: string): boolean {
    if (!line.includes("-")) return false;
    const cells = splitTableRow(line);
    return cells.length > 0 && cells.every((c) => parseSepAlign(c) !== null);
}

function isTableStart(lines: string[], i: number): boolean {
    if (i + 1 >= lines.length) return false;
    if (!lines[i].includes("|")) return false;
    if (!isTableSepRow(lines[i + 1])) return false;
    return splitTableRow(lines[i]).length === splitTableRow(lines[i + 1]).length;
}

/** 把助手回复按行拆为文本段与表格段（表格 = 表头行 + 分隔行 + 数据行）。 */
function splitGuideSegments(content: string): GuideSegment[] {
    const lines = content.replace(/\r\n/g, "\n").split("\n");
    const segs: GuideSegment[] = [];
    let buf: string[] = [];
    const flush = () => {
        const text = buf.join("\n").trim();
        if (text) segs.push({ type: "text", text });
        buf = [];
    };
    let i = 0;
    while (i < lines.length) {
        if (isTableStart(lines, i)) {
            flush();
            const header = splitTableRow(lines[i]);
            const aligns = splitTableRow(lines[i + 1]).map((c) => parseSepAlign(c) ?? undefined);
            const rows: string[][] = [];
            i += 2;
            while (i < lines.length && lines[i].trim() && lines[i].includes("|")) {
                const cells = splitTableRow(lines[i]);
                while (cells.length < header.length) cells.push("");
                rows.push(cells.slice(0, header.length));
                i += 1;
            }
            segs.push({ type: "table", header, aligns, rows });
            continue;
        }
        buf.push(lines[i]);
        i += 1;
    }
    flush();
    return segs;
}

/** 单元格行内格式：仅 **加粗** 与 `代码` 两类（导购表格内的实际用法），其余原文。 */
function renderGuideInline(text: string, keyPrefix: string): ReactNode[] {
    const parts = text.split(/(\*\*[^*\n]+\*\*|`[^`\n]+`)/g).filter((p) => p !== "");
    return parts.map((p, idx) => {
        if (p.startsWith("**") && p.endsWith("**") && p.length > 4) {
            return (
                <Text key={`${keyPrefix}-${idx}`} className="guide-md-bold">
                    {p.slice(2, -2)}
                </Text>
            );
        }
        if (p.startsWith("`") && p.endsWith("`") && p.length > 2) {
            return (
                <Text key={`${keyPrefix}-${idx}`} className="guide-md-code">
                    {p.slice(1, -1)}
                </Text>
            );
        }
        return <Text key={`${keyPrefix}-${idx}`}>{p}</Text>;
    });
}

/** 列宽估算：全角字按 1 单位、半角按 0.55 单位，24rpx 字号下约 25rpx/单位，夹在 120~440rpx。 */
function guideCellUnits(s: string): number {
    let u = 0;
    for (const ch of s) u += /[^\x00-\xff]/.test(ch) ? 1 : 0.55;
    return u;
}

function GuideTable({ seg }: { seg: Extract<GuideSegment, { type: "table" }> }) {
    const widths = seg.header.map((h, j) => {
        const maxU = Math.max(guideCellUnits(h), ...seg.rows.map((r) => guideCellUnits(r[j] || "")));
        return Math.min(440, Math.max(120, Math.ceil(maxU * 25) + 28));
    });
    const total = widths.reduce((a, b) => a + b, 0);
    const renderRow = (cells: string[], rowIdx: number, isHead: boolean) => (
        <View key={rowIdx} className={`guide-table__row${isHead ? " guide-table__row--head" : ""}`}>
            {cells.map((c, j) => (
                <View key={j} className="guide-table__cell" style={{ width: `${widths[j]}rpx` }}>
                    <Text
                        className={`guide-table__celltext${isHead ? " guide-table__celltext--head" : ""}`}
                        style={{ textAlign: seg.aligns[j] || "left" }}
                    >
                        {renderGuideInline(c, `r${rowIdx}c${j}`)}
                    </Text>
                </View>
            ))}
        </View>
    );
    return (
        <ScrollView className="guide-tablewrap" scrollX>
            <View className="guide-table" style={{ width: `${total}rpx` }}>
                {renderRow(seg.header, -1, true)}
                {seg.rows.map((r, i) => renderRow(r, i, false))}
            </View>
        </ScrollView>
    );
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
                    <FormatPrice displayPrice priceData={card.productPrice} />
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
                                        splitGuideSegments(m.content).map((seg, si) =>
                                            seg.type === "text" ? (
                                                <Text key={si}>{seg.text}</Text>
                                            ) : (
                                                <GuideTable key={si} seg={seg} />
                                            ),
                                        )
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
