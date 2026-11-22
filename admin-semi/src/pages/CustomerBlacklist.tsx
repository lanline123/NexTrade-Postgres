import { useRef, useState } from 'react';
import {
  Avatar,
  Button,
  Modal,
  Space,
  Tag,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiPost, getToken, isDemo } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import BlacklistAddModal from '../components/BlacklistAddModal';
import BlacklistImportModal from '../components/BlacklistImportModal';
import BlacklistDetailDrawer, {
  blacklistScopeLabel,
  formatBlacklistTime,
  BLACKLIST_BEHAVIOR_LABELS,
  BLACKLIST_PRODUCT_LABELS,
  type BlacklistEntryRow,
} from '../components/BlacklistDetailDrawer';

const { Text } = Typography;

/** 移除确认弹窗（单个/批量共用，带移除原因输入）。模块顶层定义，勿内嵌。 */
function BlacklistRemoveModal({
  visible,
  targets,
  onCancel,
  onSuccess,
}: {
  visible: boolean;
  targets: BlacklistEntryRow[];
  onCancel: () => void;
  onSuccess: () => void;
}) {
  const { t } = useI18n();
  const [reason, setReason] = useState('');
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    if (isDemo()) {
      Toast.warning({ content: t('演示模式不支持操作') });
      return;
    }
    setSaving(true);
    try {
      if (targets.length === 1) {
        await apiPost('/user/blacklist/remove', {
          id: targets[0].id,
          removeReason: reason.trim() || undefined,
        });
        Toast.success({ content: t('已移除') });
      } else {
        const result = await apiPost<{ successCount: number; failures: Array<{ msg: string }> }>(
          '/user/blacklist/batchRemove',
          { ids: targets.map((row) => row.id), removeReason: reason.trim() || undefined },
        );
        const failCount = result?.failures?.length || 0;
        if (failCount > 0) {
          Toast.warning({
            content: t('成功 {ok} 条，失败 {fail} 条', {
              ok: result?.successCount ?? 0,
              fail: failCount,
            }),
          });
        } else {
          Toast.success({ content: t('已移除') });
        }
      }
      setReason('');
      onSuccess();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('移除失败') });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={t('移除黑名单')}
      visible={visible}
      onCancel={onCancel}
      width={480}
      footer={
        <Space>
          <Button onClick={onCancel}>{t('取消')}</Button>
          <Button theme="solid" type="primary" loading={saving} onClick={submit}>
            {t('确认移除')}
          </Button>
        </Space>
      }
    >
      <div style={{ display: 'grid', gap: 12 }}>
        <Text>
          {targets.length > 1
            ? t('已选择 {count} 条记录，移除后对应限制立即失效。', { count: targets.length })
            : t('移除后对应限制立即失效，操作记录会保留。')}
        </Text>
        <TextArea
          placeholder={t('移除原因（选填）')}
          value={reason}
          onChange={setReason}
          maxCount={500}
          rows={2}
        />
      </div>
    </Modal>
  );
}

const buildColumns = (
  t: (key: string, params?: Record<string, string | number>) => string,
): ColumnConfig<BlacklistEntryRow>[] => [
  {
    title: '客户',
    dataIndex: 'userId',
    width: 220,
    render: (_: unknown, record) => (
      <div className="member-cell">
        {record.user?.avatar ? (
          <Avatar src={record.user.avatar} size="small" />
        ) : (
          <Avatar color="blue" size="small">
            {(record.user?.nickname || record.user?.username || 'U').slice(0, 1).toUpperCase()}
          </Avatar>
        )}
        <div>
          <div>{record.user?.nickname || record.user?.username || '-'}</div>
          <Text type="tertiary" size="small">
            {record.user?.mobile || ''} · ID {record.userId}
          </Text>
        </div>
      </div>
    ),
  },
  {
    title: '限制规则',
    dataIndex: 'rules',
    width: 220,
    render: (_: unknown, record) => (
      <Space wrap>
        {record.rules.map((rule) => (
          <Tag key={`${rule.product}:${rule.behavior}`} color="red" size="small">
            {t(BLACKLIST_PRODUCT_LABELS[rule.product] || rule.product)}-
            {t(BLACKLIST_BEHAVIOR_LABELS[rule.behavior] || rule.behavior)}
          </Tag>
        ))}
      </Space>
    ),
  },
  {
    title: '限制范围',
    dataIndex: 'applyMode',
    width: 160,
    render: (_: unknown, record) => blacklistScopeLabel(record, t),
  },
  {
    title: '归属',
    dataIndex: 'ownerType',
    width: 110,
    render: (_: unknown, record) =>
      record.ownerType === 1 ? (
        <Tag color="violet">{t('平台')}</Tag>
      ) : (
        <Tag color="blue">{record.ownerShopName || t('店铺')}</Tag>
      ),
  },
  { title: '原因', dataIndex: 'reason', width: 200 },
  { title: '操作人', dataIndex: 'createAdminName', width: 110 },
  {
    title: '加入时间',
    dataIndex: 'createTime',
    width: 150,
    render: (value: number) => formatBlacklistTime(value),
  },
  {
    title: '状态',
    dataIndex: 'status',
    width: 90,
    render: (value: number) =>
      value === 1 ? <Tag color="red">{t('生效中')}</Tag> : <Tag color="grey">{t('已移除')}</Tag>,
  },
];

/** 客户黑名单列表页（方案 §6 #1）：筛选/分页/批量移除/详情抽屉/加入弹窗。 */
export default function CustomerBlacklist() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [addVisible, setAddVisible] = useState(false);
  const [importVisible, setImportVisible] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [detailEntry, setDetailEntry] = useState<BlacklistEntryRow | null>(null);
  const [removeTargets, setRemoveTargets] = useState<BlacklistEntryRow[]>([]);
  /** 批量栏的清选回调：原先点「批量移除」在确认弹窗弹出前就 clear()，
   * 用户取消后勾选全丢——改为移除成功后才清选（回调由 batchActions 渲染时登记）。 */
  const clearSelectionRef = useRef<(() => void) | null>(null);

  /**
   * 导出 xlsx。注意：ListPage 框架不向页面暴露当前筛选值（filterValues 为
   * 框架内部 state，toolbarExtra 是静态节点），且筛选类型无日期范围控件、
   * 无值变换钩子——故本按钮导出当前账号可见范围全量，后端 export 端点
   * 本身与 list 同筛选参数（含加入时间范围），可按筛选直调。框架支持
   * 暴露筛选后再改为按当前筛选导出。
   */
  const exportEntries = async () => {
    setExporting(true);
    try {
      const res = await fetch('/adminapi/user/blacklist/export', {
        headers: { Authorization: `Bearer ${getToken()}` },
      });
      if (!res.ok) throw new Error(t('导出失败'));
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = '客户黑名单.xlsx';
      link.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('导出失败') });
    } finally {
      setExporting(false);
    }
  };

  const openRemove = (rows: BlacklistEntryRow[]) => {
    const removable = rows.filter((row) => row.canRemove && row.status === 1);
    if (!removable.length) {
      Toast.warning({ content: t('所选记录不可移除') });
      return;
    }
    if (removable.length < rows.length) {
      Toast.warning({ content: t('部分记录不可移除，已自动排除') });
    }
    setRemoveTargets(removable);
  };

  return (
    <>
      <ListPage<BlacklistEntryRow>
        key={reloadKey}
        api="/user/blacklist/list"
        rowKey="id"
        pageSize={10}
        showEdit={false}
        actionWidth={160}
        selectable
        filterLayout="popover"
        batchActions={(rows, clear) => {
          clearSelectionRef.current = clear;
          return (
            <Space>
              <Button size="small" type="danger" onClick={() => openRemove(rows)}>
                {t('批量移除')}
              </Button>
            </Space>
          );
        }}
        toolbarExtra={
          <Space>
            <Button theme="solid" type="primary" onClick={() => setAddVisible(true)}>
              {t('加入黑名单')}
            </Button>
            <Button onClick={() => setImportVisible(true)}>{t('批量导入')}</Button>
            <Button loading={exporting} onClick={exportEntries}>
              {t('导出')}
            </Button>
          </Space>
        }
        filters={[
          {
            field: 'keyword',
            label: '客户信息',
            type: 'input',
            placeholder: '客户名称 / 手机号 / 邮箱',
            width: 240,
          },
          { field: 'userId', label: '会员ID', type: 'number', width: 120 },
          {
            field: 'productCode',
            label: '限制产品',
            type: 'select',
            width: 130,
            options: [
              { label: '商城', value: 'mall' },
              { label: '分销', value: 'distribution' },
              { label: '买家秀', value: 'buyer_show' },
            ],
          },
          {
            field: 'behaviorCode',
            label: '限制行为',
            type: 'select',
            width: 150,
            options: [
              { label: '下单', value: 'place_order' },
              { label: '评价', value: 'product_comment' },
              { label: '自助成为分销员', value: 'become_salesman' },
              { label: '佣金发放', value: 'commission_grant' },
              { label: '买家秀发布', value: 'buyer_show_publish' },
              { label: '买家秀评论', value: 'buyer_show_comment' },
            ],
          },
          {
            field: 'ownerType',
            label: '归属',
            type: 'select',
            width: 120,
            options: [
              { label: '平台', value: 1 },
              { label: '店铺', value: 2 },
            ],
          },
          {
            field: 'status',
            label: '状态',
            type: 'select',
            width: 120,
            options: [
              { label: '生效中', value: 1 },
              { label: '已移除', value: 0 },
            ],
          },
        ]}
        columns={buildColumns(t)}
        rowActions={(record) => (
          <Space spacing={2}>
            <Button theme="borderless" size="small" onClick={() => setDetailEntry(record)}>
              {t('详情')}
            </Button>
            <Button
              theme="borderless"
              size="small"
              type="danger"
              disabled={!record.canRemove || record.status !== 1}
              title={
                record.ownerType === 2 && record.canRemove === false
                  ? t('店铺黑名单由店铺自行移除')
                  : undefined
              }
              onClick={() => openRemove([record])}
            >
              {t('移除')}
            </Button>
          </Space>
        )}
      />

      <BlacklistAddModal
        visible={addVisible}
        onCancel={() => setAddVisible(false)}
        onSuccess={() => {
          setAddVisible(false);
          setReloadKey((key) => key + 1);
        }}
      />

      <BlacklistImportModal
        visible={importVisible}
        onCancel={() => setImportVisible(false)}
        onSuccess={() => setReloadKey((key) => key + 1)}
      />

      <BlacklistDetailDrawer
        visible={detailEntry !== null}
        entry={detailEntry}
        onCancel={() => setDetailEntry(null)}
      />

      <BlacklistRemoveModal
        visible={removeTargets.length > 0}
        targets={removeTargets}
        onCancel={() => setRemoveTargets([])}
        onSuccess={() => {
          setRemoveTargets([]);
          clearSelectionRef.current?.();
          setReloadKey((key) => key + 1);
        }}
      />
    </>
  );
}
