/**
 * 客服工作台 · H5 移动端形态（KefuH5Workbench）。
 *
 * 论坛帖 477700「在线客服 H5 适配」（我方新增形态）：客服工作台此前只有
 * PC 后台形态（KefuWorkbench），商家/客服在手机浏览器上无法接待。本页是
 * 同一工作台的移动端形态：独立全屏页 `?page=kefuH5`（对齐 imWorkbench 的
 * 独立页模式），商家/客服用手机浏览器登录后台即可接待。
 *
 * 后端契约与 PC 版完全一致（root/im 既有接口，不新造消息通道、不新增后端）：
 * - GET  /im/conversation/conversation/list?status=1|2   会话列表（进行中/已结束）
 * - GET  /im/conversation/conversation/waitServantList   待接入队列
 * - POST /im/conversation/conversation/transfer          接入会话（type=autoTransfer）
 * - POST /im/conversation/conversation/updateField       结束会话（field=status，val=2）
 * - GET  /im/conversation/message/list                   会话消息
 * - POST /im/conversation/message/send                   发送消息（服务端覆盖 type/servantId）
 * - POST /im/conversation/message/setRead                设置已读
 * - GET  /im/servant/servant/getStatus                   客服状态（1 在线 / 2 忙 / 3 离开）
 * - POST /im/servant/servant/modifyStatus                修改客服状态
 *
 * 权限与 PC 版同源：页面在后台登录态之后渲染（App authed 门），接口走同一
 * 管理员令牌 + tenant_scope 层，客服身份即客服账号绑定，不另设权限。
 *
 * 实时性与 PC 版同口径：WebSocket 尽力而为（/ws?token&platform=admin），
 * 失败/无推送时 5s 轮询兜底。未读提醒：列表角标 + 标题栏总未读 +
 * document.title 前缀计数 + 新消息提示音（WebAudio 合成，同 PC 版）+
 * 页面回到前台（visibilitychange）立即刷新。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Modal, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost, getToken } from '../api';
import { emojiUrl } from '../lib/emojiUrls';
import { useI18n } from '../i18n';
import './KefuH5.css';

/* ============================== 类型（与 KefuWorkbench 同 VO 口径） ============================== */

interface ImUserVO {
  userId?: number;
  username?: string;
  nickname?: string;
  avatar?: string;
}

interface ContentVO {
  messageType?: string;
  content?: string;
  pic?: string;
  videoUrl?: string;
  videoCover?: string;
  contentCategory?: string;
  order?: {
    orderId?: number;
    orderSn?: string;
    picUrl?: string;
    productName?: string;
    productNum?: number;
    totalAmount?: number | string;
    orderStatusName?: string;
  };
  product?: {
    productId?: number;
    picUrl?: string;
    productName?: string;
    productSn?: string;
    productPrice?: number | string;
  };
}

interface ConversationRow {
  id?: number;
  conversationId?: number;
  userId?: number;
  lastServantId?: number;
  addTime?: string;
  lastUpdateTime?: string;
  userFrom?: string;
  status?: number;
  remark?: string;
  unreadMessageCount?: number;
  unreadServantMessageCount?: number;
  user?: ImUserVO | null;
  lastMessage?: Array<{ content?: ContentVO; sendTime?: string; type?: number }> | null;
}

interface MessageRow {
  id?: number;
  conversationId?: number;
  content?: ContentVO;
  type?: number;
  userId?: number;
  servantId?: number;
  sendTime?: string;
  isRead?: number;
  user?: { username?: string; nickname?: string; avatar?: string } | null;
  servant?: { username?: string; avatar?: string } | null;
}

/* ============================== 表情（与 PC 版同码表/同渲染规则） ============================== */

/** 旧版表情码表（与 KefuWorkbench 的 EMOJI_CODES 逐字一致，139 项）。 */
const EMOJI_CODES: Array<[string, string]> = [
  ['微笑', '1'], ['龇牙', '2'], ['色', '3'], ['发呆', '4'], ['可怜', '5'],
  ['流泪', '6'], ['害羞', '7'], ['闭嘴', '8'], ['睡', '9'], ['吃瓜群众', '10'],
  ['尴尬', '11'], ['发怒', '12'], ['调皮', '13'], ['撇嘴', '14'], ['思考', '15'],
  ['不失礼貌的微笑', '16'], ['奸笑', '17'], ['抓狂', '18'], ['吐', '19'], ['偷笑', '20'],
  ['愉快', '21'], ['白眼', '22'], ['傲慢', '23'], ['困', '24'], ['灵光一现', '25'],
  ['流汗', '26'], ['憨笑', '27'], ['捂脸', '28'], ['奋斗', '29'], ['咒骂', '30'],
  ['疑问', '31'], ['嘘', '32'], ['晕', '33'], ['衰', '34'], ['骷髅', '35'],
  ['敲打', '36'], ['再见', '37'], ['擦汗', '38'], ['抠鼻', '39'], ['泣不成声', '40'],
  ['坏笑', '41'], ['左哼哼', '42'], ['右哼哼', '43'], ['打哈欠', '44'], ['鄙视', '45'],
  ['委屈', '46'], ['快哭了', '47'], ['摸头', '48'], ['阴险', '49'], ['亲亲', '50'],
  ['机智', '51'], ['得意', '52'], ['大金牙', '53'], ['拥抱', '54'], ['大笑', '55'],
  ['送心', '56'], ['震惊', '57'], ['酷拽', '58'], ['尬笑', '59'], ['大哭', '60'],
  ['哭笑', '61'], ['做鬼脸', '62'], ['红脸', '63'], ['鼓掌', '64'], ['恐惧', '65'],
  ['斜眼', '66'], ['嘿哈', '67'], ['惊讶', '68'], ['绝望的凝视', '69'], ['囧', '70'],
  ['皱眉', '71'], ['耶', '72'], ['石化', '73'], ['我想静静', '74'], ['吐血', '75'],
  ['互粉', '76'], ['互相关注', '77'], ['加好友', '78'], ['强', '79'], ['钱', '80'],
  ['飞吻', '81'], ['打脸', '82'], ['惊恐', '83'], ['悠闲', '84'], ['泪奔', '85'],
  ['舔屏', '86'], ['紫薇别走', '87'], ['听歌', '88'], ['难过', '89'], ['生病', '90'],
  ['绿帽子', '91'], ['如花', '92'], ['惊喜', '93'], ['吐彩虹', '94'], ['吐舌', '95'],
  ['无辜呆', '96'], ['看', '97'], ['白眼的狗', '98'], ['黑脸', '99'], ['猪头', '100'],
  ['熊吉', '101'], ['不看', '102'], ['玫瑰', '103'], ['凋谢', '104'], ['嘴唇', '105'],
  ['爱心', '106'], ['心碎', '107'], ['赞', '108'], ['弱', '109'], ['握手', '110'],
  ['ok', '111'], ['谢谢', '112'], ['比心', '113'], ['碰拳', '114'], ['击掌', '115'],
  ['左', '116'], ['右', '117'], ['力量', '118'], ['胜利', '119'], ['抱拳', '120'],
  ['勾引', '121'], ['拳头', '122'], ['庆祝', '123'], ['礼物', '124'], ['红包', '125'],
  ['18禁', '126'], ['去污粉', '127'], ['666', '128'], ['给力', '129'], ['v5', '130'],
  ['菜刀', '131'], ['炸弹', '132'], ['便便', '133'], ['月亮', '134'], ['太阳', '135'],
  ['发', '136'], ['黄瓜', '137'], ['西瓜', '138'], ['啤酒', '139'],
];

const EMOJI_ID_BY_NAME = new Map<string, string>(EMOJI_CODES);

interface EmojiSegment {
  type: 'text' | 'emoji';
  value: string;
}

function splitEmojiSegments(input: string): EmojiSegment[] {
  const segments: EmojiSegment[] = [];
  const re = /\[([^\[\]]+)\]/g;
  let last = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(input))) {
    if (match.index > last) segments.push({ type: 'text', value: input.slice(last, match.index) });
    const id = EMOJI_ID_BY_NAME.get(match[1]);
    if (id) segments.push({ type: 'emoji', value: id });
    else segments.push({ type: 'text', value: match[0] });
    last = re.lastIndex;
  }
  if (last < input.length) segments.push({ type: 'text', value: input.slice(last) });
  return segments;
}

const EmojiText = ({ text }: { text: string }) => {
  const segments = splitEmojiSegments(text || '');
  return (
    <>
      {segments.map((segment, index) =>
        segment.type === 'emoji' ? (
          <img key={index} className="kefu-h5-emoji" src={emojiUrl(segment.value)} alt="" />
        ) : (
          <span key={index}>{segment.value}</span>
        ),
      )}
    </>
  );
};

/* ============================== 小工具 ============================== */

type Translate = (key: string, params?: Record<string, string | number>) => string;

/** 列表时间：今天只显示 HH:mm，其余显示 MM-DD HH:mm（sendTime 为后端字符串）。 */
function formatListTime(raw?: string): string {
  if (!raw) return '';
  const d = new Date(raw.replace(/-/g, '/'));
  if (Number.isNaN(d.getTime())) return raw.slice(5, 16);
  const pad = (n: number) => String(n).padStart(2, '0');
  const hm = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  const now = new Date();
  if (d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate()) {
    return hm;
  }
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${hm}`;
}

function lastMessagePreview(row: ConversationRow, t: Translate): string {
  const list = row.lastMessage || [];
  const last = list[list.length - 1];
  if (!last?.content) return t('暂无消息');
  const content = last.content;
  const type = (content.messageType || '').toLowerCase();
  if (type === 'image') return `[${t('图片')}]`;
  if (type === 'video') return `[${t('视频')}]`;
  if (type === 'sound') return `[${t('语音')}]`;
  if (type === 'file') return `[${t('文件')}]`;
  if (content.product) return `[${t('商品')}] ${content.product.productName || ''}`;
  if (content.order) return `[${t('订单')}] ${content.order.orderSn || ''}`;
  return content.content || t('暂无消息');
}

const STATUS_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 1, label: '在线' },
  { value: 2, label: '忙碌' },
  { value: 3, label: '离开' },
];

/** 消息气泡内容（渲染口径与 PC 版 MessageBubbleContent 对齐，尺寸按移动端收敛）。 */
const BubbleContent = ({ content }: { content?: ContentVO }) => {
  const { t } = useI18n();
  if (!content) return <span>--</span>;
  const type = (content.messageType || '').toLowerCase();
  if (type === 'image' && content.pic) {
    return <img className="kefu-h5-bubble-img" src={content.pic} alt="" />;
  }
  if (type === 'video' && content.videoUrl) {
    return <video className="kefu-h5-bubble-video" src={content.videoUrl} controls poster={content.videoCover} />;
  }
  if (type === 'sound') {
    return (
      <span className="kefu-h5-bubble-sound">
        {t('[语音消息]')}
        {content.content ? <audio src={content.content} controls style={{ height: 32, maxWidth: 200 }} /> : null}
      </span>
    );
  }
  if (type === 'file') {
    return <span>{t('[文件]')} {content.content}</span>;
  }
  if (content.product) {
    return (
      <div className="kefu-h5-card">
        {content.product.picUrl ? <img src={content.product.picUrl} alt="" /> : null}
        <div className="kefu-h5-card-main">
          <div className="kefu-h5-card-title">{content.product.productName || '--'}</div>
          <div className="kefu-h5-card-price">¥{Number(content.product.productPrice || 0).toFixed(2)}</div>
        </div>
      </div>
    );
  }
  if (content.order) {
    return (
      <div className="kefu-h5-card">
        {content.order.picUrl ? <img src={content.order.picUrl} alt="" /> : null}
        <div className="kefu-h5-card-main">
          <div className="kefu-h5-card-title">{content.order.productName || content.order.orderSn || '--'}</div>
          <div className="kefu-h5-card-sub">
            {t('订单号')}：{content.order.orderSn || '--'} · {content.order.orderStatusName || ''}
          </div>
          <div className="kefu-h5-card-price">¥{Number(content.order.totalAmount || 0).toFixed(2)}</div>
        </div>
      </div>
    );
  }
  return (
    <span style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
      <EmojiText text={content.content || ''} />
    </span>
  );
};

/* ============================== H5 客服工作台 ============================== */

type ListTab = 'active' | 'queue' | 'ended';

export function KefuH5Workbench() {
  const { t } = useI18n();

  const [servantStatus, setServantStatus] = useState<number | undefined>(undefined);
  const [tab, setTab] = useState<ListTab>('active');
  const [conversations, setConversations] = useState<ConversationRow[]>([]);
  const [queueRows, setQueueRows] = useState<ConversationRow[]>([]);
  const [listLoading, setListLoading] = useState(true);
  const [activeId, setActiveId] = useState<number | undefined>(undefined);
  const [activeFallbackName, setActiveFallbackName] = useState('');
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [msgLoading, setMsgLoading] = useState(false);
  const [draft, setDraft] = useState('');
  const [sending, setSending] = useState(false);
  const [emojiOpen, setEmojiOpen] = useState(false);
  const [endVisible, setEndVisible] = useState(false);
  const [ending, setEnding] = useState(false);

  const pollRef = useRef<number | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const bottomRef = useRef<HTMLDivElement | null>(null);
  const lastMsgIdRef = useRef(0);
  const lastMsgCountRef = useRef(0);
  const totalUnreadRef = useRef(0);
  const baseTitleRef = useRef<string>(document.title);

  /* ---------- 提示音（WebAudio 合成，同 PC 版口径；被浏览器策略拦截时静默） ---------- */
  const playMessageTone = useCallback(() => {
    try {
      const AudioCtor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AudioCtor) return;
      const ctx = new AudioCtor();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 880;
      gain.gain.setValueAtTime(0.12, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.25);
      osc.start();
      osc.stop(ctx.currentTime + 0.25);
      setTimeout(() => void ctx.close(), 400);
    } catch {
      /* 静默 */
    }
  }, []);

  /* ---------- 数据加载 ---------- */

  const loadServantStatus = useCallback(() => {
    apiGet<any>('/im/servant/servant/getStatus')
      .then((result) => {
        const status = typeof result === 'number' ? result : result?.status;
        if (status === 1 || status === 2 || status === 3) setServantStatus(status);
      })
      .catch(() => undefined);
  }, []);

  const loadConversations = useCallback(
    (silent = false) => {
      if (!silent) setListLoading(true);
      const status = tab === 'ended' ? 2 : 1;
      apiGet<any>('/im/conversation/conversation/list', { status, page: 1, size: 50 })
        .then((result) => setConversations(result?.records || []))
        .catch((error) => {
          if (!silent) Toast.error({ content: error?.message || t('加载会话列表失败') });
        })
        .finally(() => {
          if (!silent) setListLoading(false);
        });
    },
    [tab, t],
  );

  const loadQueue = useCallback(
    (silent = false) => {
      apiGet<any>('/im/conversation/conversation/waitServantList', { page: 1, size: 50 })
        .then((result) => setQueueRows(result?.records || []))
        .catch((error) => {
          if (!silent) Toast.error({ content: error?.message || t('加载待接入列表失败') });
        });
    },
    [t],
  );

  const markRead = useCallback((conversationId: number) => {
    apiPost('/im/conversation/message/setRead', { conversationId }).catch(() => undefined);
  }, []);

  const loadMessages = useCallback(
    (conversationId: number, silent = false) => {
      if (!silent) setMsgLoading(true);
      apiGet<any>('/im/conversation/message/list', {
        conversationId,
        page: 1,
        size: 100,
        sortField: 'sendTime',
        sortOrder: 'asc',
      })
        .then((result) => {
          const rows: MessageRow[] = result?.records || [];
          if (silent) {
            const latestId = rows.length ? Number(rows[rows.length - 1].id ?? 0) : 0;
            const grew = rows.length > lastMsgCountRef.current;
            const hasNewFromUser =
              grew && rows.some((msg) => Number(msg.type) !== 2 && Number(msg.id ?? 0) > lastMsgIdRef.current);
            if (hasNewFromUser) {
              playMessageTone();
              // 正在看该会话且有新客户消息：顺手置已读，避免列表未读虚高
              markRead(conversationId);
            }
            lastMsgIdRef.current = Math.max(latestId, lastMsgIdRef.current);
            lastMsgCountRef.current = rows.length;
          } else {
            lastMsgIdRef.current = rows.length ? Number(rows[rows.length - 1].id ?? 0) : 0;
            lastMsgCountRef.current = rows.length;
          }
          setMessages(rows);
        })
        .catch((error) => {
          if (!silent) Toast.error({ content: error?.message || t('加载消息失败') });
        })
        .finally(() => {
          if (!silent) setMsgLoading(false);
        });
    },
    [t, playMessageTone, markRead],
  );

  /* ---------- 实时：WS 尽力而为 + 5s 轮询 + 回到前台刷新（同 PC 版口径） ---------- */

  useEffect(() => {
    loadServantStatus();
    loadQueue(true);
    try {
      const wsParams = new URLSearchParams({ token: getToken() || '', platform: 'admin' });
      const wsUrl = `${location.origin.replace(/^http:/, 'ws:').replace(/^https:/, 'wss:')}/ws?${wsParams.toString()}`;
      const ws = new WebSocket(wsUrl);
      ws.onmessage = () => {
        loadConversations(true);
        loadQueue(true);
        if (activeId) loadMessages(activeId, true);
      };
      wsRef.current = ws;
    } catch {
      wsRef.current = null;
    }
    return () => {
      wsRef.current?.close();
      wsRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    loadConversations();
  }, [loadConversations]);

  useEffect(() => {
    pollRef.current = window.setInterval(() => {
      loadConversations(true);
      loadQueue(true);
      if (activeId) loadMessages(activeId, true);
    }, 5000);
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        loadConversations(true);
        loadQueue(true);
        if (activeId) loadMessages(activeId, true);
      }
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      if (pollRef.current) window.clearInterval(pollRef.current);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [activeId, loadConversations, loadQueue, loadMessages]);

  /* ---------- 未读总数 → 标题计数（列表页/聊天页都生效） ---------- */

  const totalUnread = useMemo(
    () =>
      conversations.reduce(
        (sum, row) => sum + (tab === 'ended' ? 0 : Number(row.unreadMessageCount || 0)),
        0,
      ),
    [conversations, tab],
  );

  useEffect(() => {
    if (totalUnread > totalUnreadRef.current && activeId === undefined) playMessageTone();
    totalUnreadRef.current = totalUnread;
    document.title = totalUnread > 0 ? `(${totalUnread}) ${baseTitleRef.current}` : baseTitleRef.current;
  }, [totalUnread, activeId, playMessageTone]);

  useEffect(
    () => () => {
      document.title = baseTitleRef.current;
    },
    [],
  );

  /* ---------- 会话操作 ---------- */

  const openConversation = (row: ConversationRow) => {
    const id = row.conversationId ?? row.id;
    if (!id) return;
    setActiveId(id);
    setActiveFallbackName(row.user?.nickname || row.user?.username || '');
    setMessages([]);
    lastMsgIdRef.current = 0;
    lastMsgCountRef.current = 0;
    loadMessages(id);
    markRead(id);
  };

  const closeChat = () => {
    setActiveId(undefined);
    setMessages([]);
    setEmojiOpen(false);
    loadConversations(true);
  };

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, emojiOpen]);

  const changeStatus = async (next: number) => {
    try {
      await apiPost('/im/servant/servant/modifyStatus', { status: next });
      setServantStatus(next);
      Toast.success({ content: t('状态已更新') });
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const acceptQueue = async (row: ConversationRow) => {
    try {
      await apiPost('/im/conversation/conversation/transfer', {
        conversationId: row.conversationId ?? row.id,
        type: 'autoTransfer',
      });
      Toast.success({ content: t('接入成功') });
      setTab('active');
      loadQueue(true);
      loadConversations(true);
      openConversation(row);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('接入失败') });
    }
  };

  const endConversation = async () => {
    if (!activeId) return;
    setEnding(true);
    try {
      await apiPost('/im/conversation/conversation/updateField', { id: activeId, field: 'status', val: 2 });
      Toast.success({ content: t('会话已结束') });
      setEndVisible(false);
      closeChat();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    } finally {
      setEnding(false);
    }
  };

  const sendMessage = async () => {
    const text = draft.trim();
    if (!text || !activeId || sending) return;
    setSending(true);
    try {
      await apiPost('/im/conversation/message/send', {
        conversationId: activeId,
        content: { messageType: 'text', content: text },
      });
      setDraft('');
      loadMessages(activeId, true);
      loadConversations(true);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('发送失败') });
    } finally {
      setSending(false);
    }
  };

  const activeConversation = conversations.find((row) => (row.conversationId ?? row.id) === activeId);
  const activeName =
    activeConversation?.user?.nickname ||
    activeConversation?.user?.username ||
    activeFallbackName ||
    t('客户');
  const activeEnded = tab === 'ended' || activeConversation?.status === 2;

  /* ============================== 聊天视图 ============================== */

  if (activeId !== undefined) {
    return (
      <div className="kefu-h5">
        <div className="kefu-h5-chatbar">
          <button type="button" className="kefu-h5-iconbtn" onClick={closeChat} aria-label={t('返回')}>
            ‹
          </button>
          <div className="kefu-h5-chatbar-title">
            <span className="kefu-h5-chatbar-name">{activeName}</span>
            {activeConversation?.userFrom ? (
              <span className="kefu-h5-chatbar-sub">{activeConversation.userFrom}</span>
            ) : null}
          </div>
          {!activeEnded ? (
            <button type="button" className="kefu-h5-endbtn" onClick={() => setEndVisible(true)}>
              {t('结束会话')}
            </button>
          ) : (
            <span className="kefu-h5-ended-tag">{t('已结束')}</span>
          )}
        </div>

        <div className="kefu-h5-msgs">
          {msgLoading && messages.length === 0 ? (
            <div className="kefu-h5-empty">{t('加载中…')}</div>
          ) : messages.length === 0 ? (
            <div className="kefu-h5-empty">{t('暂无消息')}</div>
          ) : (
            messages.map((msg) => {
              const own = Number(msg.type) === 2;
              const avatar = own ? msg.servant?.avatar : msg.user?.avatar;
              const name = own ? msg.servant?.username : msg.user?.nickname || msg.user?.username;
              return (
                <div key={msg.id} className={`kefu-h5-msgrow${own ? ' own' : ''}`}>
                  {avatar ? (
                    <img className="kefu-h5-avatar" src={avatar} alt="" />
                  ) : (
                    <span className="kefu-h5-avatar kefu-h5-avatar-fallback">{(name || '?').slice(0, 1)}</span>
                  )}
                  <div className="kefu-h5-msgcol">
                    <div className="kefu-h5-msgmeta">
                      {name || ''} {msg.sendTime ? formatListTime(msg.sendTime) : ''}
                    </div>
                    <div className={`kefu-h5-bubble${own ? ' own' : ''}`}>
                      <BubbleContent content={msg.content} />
                    </div>
                  </div>
                </div>
              );
            })
          )}
          <div ref={bottomRef} />
        </div>

        {!activeEnded ? (
          <div className="kefu-h5-composer">
            <div className="kefu-h5-composer-row">
              <button
                type="button"
                className="kefu-h5-iconbtn"
                onClick={() => setEmojiOpen((v) => !v)}
                aria-label={t('表情')}
              >
                ☺
              </button>
              <textarea
                className="kefu-h5-input"
                rows={1}
                value={draft}
                placeholder={t('输入回复内容')}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    void sendMessage();
                  }
                }}
              />
              <button
                type="button"
                className="kefu-h5-sendbtn"
                disabled={!draft.trim() || sending}
                onClick={() => void sendMessage()}
              >
                {t('发送')}
              </button>
            </div>
            {emojiOpen ? (
              <div className="kefu-h5-emoji-panel">
                {EMOJI_CODES.map(([name, id]) => (
                  <button
                    key={id}
                    type="button"
                    className="kefu-h5-emoji-cell"
                    onClick={() => setDraft((d) => `${d}[${name}]`)}
                  >
                    <img src={emojiUrl(id)} alt={name} />
                  </button>
                ))}
              </div>
            ) : null}
          </div>
        ) : null}

        <Modal
          title={t('结束该会话？')}
          visible={endVisible}
          onCancel={() => setEndVisible(false)}
          onOk={() => void endConversation()}
          okText={t('结束会话')}
          cancelText={t('取消')}
          confirmLoading={ending}
        >
          <div>{t('结束后会话将移入「已结束」，客户再次咨询会开启新会话。')}</div>
        </Modal>
      </div>
    );
  }

  /* ============================== 列表视图 ============================== */

  const rows = tab === 'queue' ? queueRows : conversations;

  return (
    <div className="kefu-h5">
      <div className="kefu-h5-topbar">
        <span className="kefu-h5-title">
          {t('客服工作台')}
          {totalUnread > 0 ? <span className="kefu-h5-badge">{totalUnread > 99 ? '99+' : totalUnread}</span> : null}
        </span>
        <a className="kefu-h5-pc-link" href={`${window.location.origin}/?page=dashboard`}>
          {t('返回后台')}
        </a>
      </div>

      <div className="kefu-h5-statusbar">
        <span className="kefu-h5-status-label">{t('客服状态')}</span>
        <div className="kefu-h5-seg">
          {STATUS_OPTIONS.map((item) => (
            <button
              key={item.value}
              type="button"
              className={`kefu-h5-seg-btn${servantStatus === item.value ? ' on' : ''}`}
              onClick={() => void changeStatus(item.value)}
            >
              <span className={`kefu-h5-dot s${item.value}`} />
              {t(item.label)}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="kefu-h5-refresh"
          onClick={() => {
            loadConversations();
            loadQueue();
          }}
        >
          {t('刷新')}
        </button>
      </div>

      <div className="kefu-h5-tabs">
        <button
          type="button"
          className={`kefu-h5-tab${tab === 'active' ? ' on' : ''}`}
          onClick={() => setTab('active')}
        >
          {t('进行中')}
        </button>
        <button
          type="button"
          className={`kefu-h5-tab${tab === 'queue' ? ' on' : ''}`}
          onClick={() => setTab('queue')}
        >
          {t('待接入')}
          {queueRows.length > 0 ? <span className="kefu-h5-badge">{queueRows.length}</span> : null}
        </button>
        <button
          type="button"
          className={`kefu-h5-tab${tab === 'ended' ? ' on' : ''}`}
          onClick={() => setTab('ended')}
        >
          {t('已结束')}
        </button>
      </div>

      <div className="kefu-h5-list">
        {listLoading && tab !== 'queue' && rows.length === 0 ? (
          <div className="kefu-h5-empty">{t('加载中…')}</div>
        ) : rows.length === 0 ? (
          <div className="kefu-h5-empty">
            {tab === 'queue' ? t('暂无待接入会话') : tab === 'ended' ? t('暂无已结束会话') : t('暂无进行中会话')}
          </div>
        ) : (
          rows.map((row) => {
            const id = row.conversationId ?? row.id;
            const name = row.user?.nickname || row.user?.username || row.userId || '--';
            const unread = tab === 'active' ? Number(row.unreadMessageCount || 0) : 0;
            return (
              <div
                key={id}
                className="kefu-h5-row"
                onClick={() => {
                  if (tab !== 'queue') openConversation(row);
                }}
              >
                {row.user?.avatar ? (
                  <img className="kefu-h5-avatar lg" src={row.user.avatar} alt="" />
                ) : (
                  <span className="kefu-h5-avatar lg kefu-h5-avatar-fallback">{String(name).slice(0, 1)}</span>
                )}
                <div className="kefu-h5-row-main">
                  <div className="kefu-h5-row-top">
                    <span className="kefu-h5-row-name">{name}</span>
                    <span className="kefu-h5-row-time">{formatListTime(row.lastUpdateTime || row.addTime)}</span>
                  </div>
                  <div className="kefu-h5-row-bottom">
                    <span className="kefu-h5-row-preview">{lastMessagePreview(row, t)}</span>
                    {unread > 0 ? (
                      <span className="kefu-h5-badge">{unread > 99 ? '99+' : unread}</span>
                    ) : null}
                    {tab === 'queue' ? (
                      <button
                        type="button"
                        className="kefu-h5-accept"
                        onClick={(e) => {
                          e.stopPropagation();
                          void acceptQueue(row);
                        }}
                      >
                        {t('接入')}
                      </button>
                    ) : null}
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
