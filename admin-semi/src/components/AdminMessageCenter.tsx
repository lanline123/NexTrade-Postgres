import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { Button, Empty, Spin, Tag, Toast } from '@douyinfe/semi-ui';
import {
  IconApartment,
  IconBell,
  IconBox,
  IconCart,
  IconClose,
  IconComment,
  IconCreditCard,
  IconUndo,
} from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import { pageTitles } from '../config/adminMenu';
import './admin-message-center.css';

interface AdminMsg {
  msgId: number;
  msgType: number;
  msgTypeName: string;
  sendTime: string;
  isReaded: number;
  title: string;
  content: string;
  msgLink?: string | null;
  route?: string | null;
}

interface AdminMsgTypeChild {
  msgType: number;
  name: string;
  unreadCount: number;
}

interface AdminMsgType {
  catId: number;
  catName: string;
  unreadCount: number;
  child: AdminMsgTypeChild[];
}

export type MsgAccountKind = 'platform' | 'shop' | 'vendor';

interface AdminMessageCenterProps {
  visible: boolean;
  onClose: () => void;
  /** AI 审批中心待审批数（>0 时在面板顶部显示一条可点击的审批提醒，与菜单 pill 同源同值） */
  approvalCount?: number;
  /** 点击审批提醒条：关面板并跳到审批中心 */
  onOpenApprovals?: () => void;
  /** 账号维度（平台/店铺/供应商）：仅影响个别消息类型的跳转目标（如保证金预警各自的保证金页） */
  accountKind?: MsgAccountKind;
  /**
   * 点击消息条目的跳转回调（目标为 admin 页面 key）。
   * 布局层负责校验目标页对当前账号真实存在，不存在则不跳（条目仍会标已读）。
   */
  onNavigate?: (pageKey: string) => void;
  /**
   * 已读动作回调。
   * @param unread 已读后剩余的未读总数；传入时顶部角标立即更新，不必等接口往返。
   */
  onRead: (unread?: number) => void;
}

/** 未读数角标（复用全局 .admin-msg-num：红底白字胶囊，>99 显示 99+，与菜单 pill 同一口径） */
function UnreadPill({ count }: { count: number }) {
  const value = Number(count) || 0;
  if (value <= 0) return null;
  return <em className="admin-msg-num">{value > 99 ? '99+' : value}</em>;
}

/** 分类图标（catId 与后端 AdminMsgTypeEnum 父分类一致：1 交易 / 2 商品 / 3 售后 / 4 店铺 / 5 其它 / 6 财务） */
const CATEGORY_ICONS: Record<number, ReactNode> = {
  1: <IconCart />,
  2: <IconBox />,
  3: <IconUndo />,
  4: <IconApartment />,
  5: <IconComment />,
  6: <IconCreditCard />,
};

/**
 * 消息类型 → 业务页 key 的跳转映射（纯呈现层导航，不改计数与产生侧）。
 *
 * 为什么不用消息自带的 route/msgLink：现网全部产生侧写 admin_msg 时
 * `msg_link` 恒为空串、`related_data` 只有 `{orderId}`、没有 route
 * （后端 `applyDefaultRoute` 未迁移），两个字段都指不到真实页面，
 * 故按 msgType 映射到确实存在的列表页；映射只收口径明确的类型，
 * 提现申请（33，会员/店铺/供应商三处同名页无法区分）与系统/反馈类
 * （51/52/53/55/56/57，无对应业务页）不映射，点击只标已读。
 */
const MSG_TYPE_NAV: Record<number, string> = {
  11: 'orders', // 新订单
  12: 'orders', // 已付款订单
  13: 'orders', // 订单完成
  15: 'orders', // 订单问题
  16: 'orders', // 订单咨询
  31: 'orders', // 订单取消
  54: 'deliverManage', // 发货提醒
  21: 'stockWarningManage', // 商品库存预警
  22: 'products', // 商品无货
  23: 'products', // 商品下架
  24: 'products', // 商品审核通知
  26: 'enquiryManage', // 商品询价
  32: 'aftersalesManage', // 售后申请
  36: 'aftersalesManage', // 售后退款
  34: 'orderInvoiceManage', // 发票资质审核
  35: 'orderInvoiceManage', // 发票申请
  37: 'offlinePaymentClaimManage', // 线下支付审核
  41: 'applyManage', // 店铺入驻申请
  42: 'shopManage', // 店铺资质修改
  43: 'shopManage', // 店铺违规
};

/** 解析一条消息的跳转目标；msgLink 若本身就是真实页面 key 则优先（为未来产生侧预留） */
function navTargetFor(msg: AdminMsg, accountKind: MsgAccountKind): string | undefined {
  const link = (msg.msgLink || '').trim();
  if (link && pageTitles[link]) return link;
  // 保证金预警（44）：三类账号各有自己的保证金页
  if (msg.msgType === 44) {
    if (accountKind === 'shop') return 'shopDepositManage';
    if (accountKind === 'vendor') return 'vendorDepositManage';
    return 'depositAccountManage';
  }
  return MSG_TYPE_NAV[msg.msgType];
}

/** 相对时间：刚刚 / N分钟前 / N小时前 / N天前，超过 30 天回落日期（sendTime 为后端 +08 格式化串） */
function relativeTime(sendTime: string, t: (key: string, vars?: Record<string, string | number>) => string): string {
  const ts = Date.parse(String(sendTime || '').replace(' ', 'T'));
  if (Number.isNaN(ts)) return sendTime;
  const diff = Math.floor((Date.now() - ts) / 1000);
  if (diff < 60) return t('刚刚');
  if (diff < 3600) return t('{count}分钟前', { count: Math.floor(diff / 60) });
  if (diff < 86400) return t('{count}小时前', { count: Math.floor(diff / 3600) });
  if (diff < 86400 * 30) return t('{count}天前', { count: Math.floor(diff / 86400) });
  return String(sendTime).slice(0, 10);
}

export default function AdminMessageCenter({
  visible,
  onClose,
  onRead,
  approvalCount = 0,
  onOpenApprovals,
  accountKind = 'platform',
  onNavigate,
}: AdminMessageCenterProps) {
  const { t } = useI18n();
  const [rows, setRows] = useState<AdminMsg[]>([]);
  const [loading, setLoading] = useState(false);
  const [marking, setMarking] = useState(false);
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [types, setTypes] = useState<AdminMsgType[]>([]);
  const [catId, setCatId] = useState(-1);
  const [msgType, setMsgType] = useState(0);
  const pageSize = 15;

  /** 拉取分类树（含各分类 Tab 与子类型的未读数） */
  const loadTypes = useCallback(async () => {
    try {
      const result = await apiGet<AdminMsgType[]>('/msg/adminMsg/msgTypeArr');
      setTypes(Array.isArray(result) ? result : []);
    } catch {
      setTypes([]);
    }
  }, []);

  const load = useCallback(
    async (nextPage: number) => {
      setLoading(true);
      try {
        const result = await apiGet<{ records?: AdminMsg[]; total?: number }>('/msg/adminMsg/list', {
          page: nextPage,
          size: pageSize,
          ...(msgType > 0 ? { msgType } : {}),
        });
        const list = result?.records || [];
        setRows((current) => (nextPage === 1 ? list : [...current, ...list]));
        setTotal(Number(result?.total) || 0);
        setPage(nextPage);
      } catch {
        if (nextPage === 1) setRows([]);
      } finally {
        setLoading(false);
      }
    },
    [msgType],
  );

  useEffect(() => {
    if (!visible) return;
    setCatId(-1);
    setMsgType(0);
    void loadTypes();
    void load(1);
    // 每次打开都以服务端未读数为准，避免沿用上次的旧角标
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  useEffect(() => {
    if (visible) void load(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [msgType]);

  /**
   * 本地先把该条消息对应的分类/子类型未读数扣掉，
   * 让 Tab 角标在点击瞬间就变化，不必等接口往返。
   */
  const applyLocalRead = useCallback((msg: AdminMsg) => {
    setRows((current) => current.map((item) => (item.msgId === msg.msgId ? { ...item, isReaded: 1 } : item)));
    setTypes((current) =>
      current.map((cat) => {
        const hit = (cat.child || []).some((child) => child.msgType === msg.msgType);
        if (!hit) return cat;
        return {
          ...cat,
          unreadCount: Math.max(0, (cat.unreadCount || 0) - 1),
          child: (cat.child || []).map((child) =>
            child.msgType === msg.msgType
              ? { ...child, unreadCount: Math.max(0, (child.unreadCount || 0) - 1) }
              : child,
          ),
        };
      }),
    );
  }, []);

  const currentCat = useMemo(() => types.find((cat) => cat.catId === catId), [types, catId]);
  const totalUnread = useMemo(
    () => types.reduce((sum, cat) => sum + (Number(cat.unreadCount) || 0), 0),
    [types],
  );
  /** msgType → 父分类（图标用）：以服务端分类树为准，树未就绪时回落铃铛图标 */
  const catOfType = useMemo(() => {
    const map = new Map<number, number>();
    types.forEach((cat) => (cat.child || []).forEach((child) => map.set(child.msgType, cat.catId)));
    return map;
  }, [types]);

  /** 全部已读：本地角标先清零，再与服务端对齐 */
  const markAllRead = async () => {
    setMarking(true);
    try {
      await apiPost('/msg/adminMsg/setAllReaded', {});
      setRows((current) => current.map((item) => ({ ...item, isReaded: 1 })));
      setTypes((current) =>
        current.map((cat) => ({
          ...cat,
          unreadCount: 0,
          child: (cat.child || []).map((child) => ({ ...child, unreadCount: 0 })),
        })),
      );
      // 立刻归零，随后再按服务端真实值对齐（并发新消息时两者可能不同）
      onRead(0);
      onRead();
      void loadTypes();
      void load(1);
    } catch (error) {
      Toast.error((error as Error)?.message || t('操作失败'));
    } finally {
      setMarking(false);
    }
  };

  /** 单条已读：分类 Tab / 子类型角标与顶部角标都在点击瞬间更新 */
  const markRead = async (msg: AdminMsg) => {
    if (msg.isReaded === 1) return;
    applyLocalRead(msg);
    // 点击瞬间更新顶部角标（此时服务端尚未改完，不能立刻去拉接口，否则会被旧值顶回去）
    onRead(Math.max(0, totalUnread - 1));
    try {
      await apiPost('/msg/adminMsg/setReaded', { msgId: msg.msgId });
      // 已读成功后再按服务端真实值对齐
      onRead();
      void loadTypes();
    } catch {
      // 失败回滚：重拉分类树与列表，回到服务端真实状态
      void loadTypes();
      void load(1);
      onRead();
    }
  };

  /** 点击条目：先标已读（与旧行为一致的乐观更新），有明确业务页时再直达 */
  const handleItemClick = (msg: AdminMsg) => {
    void markRead(msg);
    const target = navTargetFor(msg, accountKind);
    if (target && onNavigate) onNavigate(target);
  };

  const selectCat = (nextCat: number) => {
    setCatId(nextCat);
    const firstChild = types.find((cat) => cat.catId === nextCat)?.child?.[0]?.msgType;
    setMsgType(nextCat > 0 && firstChild ? firstChild : 0);
  };

  const showApproval = approvalCount > 0 && !!onOpenApprovals;

  return (
    <div className="admin-msg-center">
      <div className="msgc-head">
        <div className="msgc-head__main">
          <span className="msgc-head__title">{t('消息中心')}</span>
          <span className="msgc-head__meta">
            {totalUnread > 0 ? t('{count} 条未读', { count: totalUnread }) : t('共 {total} 条消息', { total })}
          </span>
        </div>
        <div className="msgc-head__actions">
          <Button size="small" theme="borderless" loading={marking} disabled={totalUnread === 0} onClick={markAllRead}>
            {t('全部已读')}
          </Button>
          <Button
            size="small"
            theme="borderless"
            icon={<IconClose />}
            aria-label={t('关闭')}
            title={t('关闭')}
            onClick={onClose}
          />
        </div>
      </div>

      {showApproval && (
        <button type="button" className="msgc-approval" onClick={onOpenApprovals}>
          <Tag color="red" size="small">
            {t('AI 审批')}
          </Tag>
          <span className="msgc-approval__text">
            {t('AI 审批中心有 {count} 条待审批', { count: approvalCount })}
          </span>
          <span className="msgc-approval__hint">{t('点击前往审批中心处理')}</span>
        </button>
      )}

      <div className="msgc-tabs">
        <button
          type="button"
          className={`msgc-tab${catId === -1 ? ' is-active' : ''}`}
          onClick={() => selectCat(-1)}
        >
          <span>{t('全部消息')}</span>
          <UnreadPill count={totalUnread} />
        </button>
        {types.map((cat) => (
          <button
            type="button"
            key={cat.catId}
            className={`msgc-tab${catId === cat.catId ? ' is-active' : ''}`}
            onClick={() => selectCat(cat.catId)}
          >
            <span>{cat.catName}</span>
            <UnreadPill count={cat.unreadCount} />
          </button>
        ))}
      </div>

      {catId > 0 && (
        <div className="msgc-chips">
          <button
            type="button"
            className={`msgc-chip${msgType === 0 ? ' is-active' : ''}`}
            onClick={() => setMsgType(0)}
          >
            <span>{t('全部类型')}</span>
            <UnreadPill count={currentCat?.unreadCount || 0} />
          </button>
          {(currentCat?.child || []).map((child) => (
            <button
              type="button"
              key={child.msgType}
              className={`msgc-chip${msgType === child.msgType ? ' is-active' : ''}`}
              onClick={() => setMsgType(child.msgType)}
            >
              <span>{child.name}</span>
              <UnreadPill count={child.unreadCount} />
            </button>
          ))}
        </div>
      )}

      <div className="msgc-list">
        {loading && rows.length === 0 ? (
          <div className="msgc-state">
            <Spin />
          </div>
        ) : rows.length === 0 ? (
          // 有审批提醒条时不再显示空态，避免上下矛盾（与旧版口径一致）
          showApproval ? null : (
            <Empty description={t('暂无新消息')} style={{ padding: '36px 20px' }} />
          )
        ) : (
          rows.map((item) => (
            <button
              type="button"
              key={item.msgId}
              className={`msgc-item${item.isReaded === 1 ? ' is-readed' : ' is-unread'}`}
              onClick={() => handleItemClick(item)}
            >
              <span className="msgc-item__icon">
                {CATEGORY_ICONS[catOfType.get(item.msgType) ?? 0] || <IconBell />}
              </span>
              <span className="msgc-item__body">
                <span className="msgc-item__row">
                  <span className="msgc-item__title">{item.title}</span>
                  <span className="msgc-item__time" title={item.sendTime}>
                    {relativeTime(item.sendTime, t)}
                  </span>
                </span>
                <span className="msgc-item__row msgc-item__row--sub">
                  <Tag color={item.isReaded === 1 ? 'grey' : 'blue'} size="small">
                    {item.msgTypeName || t('消息')}
                  </Tag>
                  <span className="msgc-item__content">{item.content}</span>
                </span>
              </span>
              {item.isReaded !== 1 && <span className="msgc-item__dot" />}
            </button>
          ))
        )}
      </div>

      {rows.length < total && (
        <div className="msgc-more">
          <Button size="small" theme="borderless" loading={loading} onClick={() => void load(page + 1)}>
            {t('加载更多')}
          </Button>
        </div>
      )}
    </div>
  );
}
