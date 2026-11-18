import { useEffect, useState } from 'react';
import { Button, Descriptions, SideSheet, Space, Tag, Timeline, Typography } from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

export interface BlacklistEntryRow {
  id: number;
  userId: number;
  user?: {
    userId: number;
    username?: string | null;
    nickname?: string | null;
    mobile?: string | null;
    avatar?: string | null;
  } | null;
  ownerType: number;
  ownerShopId: number;
  ownerShopName?: string | null;
  applyMode: number;
  shopIds: number[];
  shopNames: string[];
  rules: Array<{ product: string; behavior: string }>;
  reason: string;
  source: number;
  status: number;
  createAdminName: string;
  createAdminType: string;
  createTime: number;
  removeAdminName: string;
  removeTime: number;
  removeReason: string;
  canRemove: boolean;
}

interface BlacklistLogRow {
  id: number;
  opType: number;
  opContent: string;
  operatorName: string;
  operatorType: string;
  createTime: number;
  rulesSnapshot?: Array<{ product: string; behavior: string }>;
}

export const BLACKLIST_PRODUCT_LABELS: Record<string, string> = {
  mall: '商城',
  distribution: '分销',
  buyer_show: '买家秀',
};

export const BLACKLIST_BEHAVIOR_LABELS: Record<string, string> = {
  place_order: '下单',
  product_comment: '评价',
  become_salesman: '自助成为分销员',
  commission_grant: '佣金发放',
  buyer_show_publish: '买家秀发布',
  buyer_show_comment: '买家秀评论',
};

const OP_TYPE_LABELS: Record<number, string> = {
  1: '加入',
  2: '移除',
  3: '批量加入',
  4: '批量移除',
  5: '导入加入',
  6: '修改规则',
  7: '开关变更',
};

export function formatBlacklistTime(seconds: number): string {
  if (!seconds) return '-';
  const date = new Date(seconds * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function blacklistScopeLabel(
  entry: BlacklistEntryRow,
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  if (entry.ownerType === 2) return entry.ownerShopName || t('本店');
  if (entry.applyMode === 1) {
    return entry.shopNames.length
      ? entry.shopNames.join('、')
      : t('指定店铺 {count} 家', { count: entry.shopIds.length });
  }
  return t('全平台');
}

/**
 * 黑名单详情抽屉：上 = 条目信息，下 = 操作记录时间线（logList 按 blacklistId）。
 * 组件定义在模块顶层（避免 React remount，见 AGENTS.md 教训）。
 */
export default function BlacklistDetailDrawer({
  visible,
  entry,
  onCancel,
}: {
  visible: boolean;
  entry: BlacklistEntryRow | null;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [detail, setDetail] = useState<BlacklistEntryRow | null>(null);
  const [logs, setLogs] = useState<BlacklistLogRow[]>([]);

  useEffect(() => {
    if (!visible || !entry) return;
    let alive = true;
    apiGet<BlacklistEntryRow>('/user/blacklist/detail', { id: entry.id })
      .then((result) => alive && setDetail(result))
      .catch(() => alive && setDetail(entry));
    apiGet<{ records?: BlacklistLogRow[] }>('/user/blacklist/logList', {
      blacklistId: entry.id,
      size: 100,
    })
      .then((result) => alive && setLogs(result?.records || []))
      .catch(() => alive && setLogs([]));
    return () => {
      alive = false;
    };
  }, [visible, entry]);

  const data: BlacklistEntryRow | null = detail || entry;
  const user = data?.user;

  return (
    <SideSheet
      title={t('黑名单详情')}
      visible={visible}
      onCancel={onCancel}
      width={640}
      placement="right"
      bodyStyle={{ padding: '16px 24px', overflowY: 'auto' }}
      footer={
        <Space>
          <Button onClick={onCancel}>{t('关闭')}</Button>
        </Space>
      }
    >
      {data && (
        <div style={{ display: 'grid', gap: 20 }}>
          <Descriptions
            row
            data={[
              {
                key: t('客户'),
                value: user
                  ? `${user.nickname || user.username || ''}（${user.mobile || ''}，ID ${data.userId}）`
                  : `ID ${data.userId}`,
              },
              {
                key: t('限制规则'),
                value: (
                  <Space wrap>
                    {data.rules.map((rule) => (
                      <Tag key={`${rule.product}:${rule.behavior}`} color="red">
                        {t(BLACKLIST_PRODUCT_LABELS[rule.product] || rule.product)}-
                        {t(BLACKLIST_BEHAVIOR_LABELS[rule.behavior] || rule.behavior)}
                      </Tag>
                    ))}
                  </Space>
                ),
              },
              { key: t('限制范围'), value: blacklistScopeLabel(data, t) },
              {
                key: t('归属'),
                value: data.ownerType === 1 ? t('平台') : data.ownerShopName || t('店铺'),
              },
              { key: t('限制原因'), value: data.reason || '-' },
              {
                key: t('状态'),
                value:
                  data.status === 1 ? (
                    <Tag color="red">{t('生效中')}</Tag>
                  ) : (
                    <Tag color="grey">{t('已移除')}</Tag>
                  ),
              },
              {
                key: t('加入信息'),
                value: `${data.createAdminName || '-'} · ${formatBlacklistTime(data.createTime)}`,
              },
              ...(data.status !== 1
                ? [
                    {
                      key: t('移除信息'),
                      value: `${data.removeAdminName || '-'} · ${formatBlacklistTime(data.removeTime)}${data.removeReason ? ` · ${data.removeReason}` : ''}`,
                    },
                  ]
                : []),
            ]}
          />
          <div>
            <Text strong>{t('操作记录')}</Text>
            {logs.length ? (
              <Timeline style={{ marginTop: 12 }}>
                {logs.map((log) => (
                  <Timeline.Item
                    key={log.id}
                    time={formatBlacklistTime(log.createTime)}
                    extra={log.operatorName || undefined}
                  >
                    <Space>
                      <Tag color="blue">{t(OP_TYPE_LABELS[log.opType] || '操作')}</Tag>
                      <span>{log.opContent || '-'}</span>
                    </Space>
                    {Array.isArray(log.rulesSnapshot) && log.rulesSnapshot.length > 0 && (
                      <div>
                        <Text type="tertiary" size="small">
                          {log.rulesSnapshot
                            .map(
                              (rule) =>
                                `${BLACKLIST_PRODUCT_LABELS[rule.product] || rule.product}-${BLACKLIST_BEHAVIOR_LABELS[rule.behavior] || rule.behavior}`,
                            )
                            .join('、')}
                        </Text>
                      </div>
                    )}
                  </Timeline.Item>
                ))}
              </Timeline>
            ) : (
              <div style={{ marginTop: 12 }}>
                <Text type="tertiary">{t('暂无操作记录')}</Text>
              </div>
            )}
          </div>
        </div>
      )}
    </SideSheet>
  );
}
