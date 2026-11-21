// 我方新增（万象互联三期⑤）：联邦结算页（万象互联 → 联邦结算）。
// 只记账、只生成结算单，不做真实跨站资金划拨：跨店分账只记账目（渠道未执行），
// 「结清登记」只改状态。预付账户余额可扣成负，负额/低于预警线在账户页告警。
import { useState } from 'react';
import { Button, DatePicker, InputNumber, Modal, Radio, RadioGroup, Select, Space, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface AccountRow {
  id: number;
  subjectType: string;
  subjectId: number;
  balance?: number | string;
  warnLine?: number | string;
  warning?: boolean;
  status: number;
  updateTime?: number;
}

interface LineRow {
  id: number;
  linkId: number;
  sourceType: string;
  remoteOrderSn?: string;
  itemName?: string;
  quantity: number;
  dealAmount?: number | string;
  supplyAmount?: number | string;
  rate?: number | string;
  ratePending: number;
  selfDue?: number | string;
  peerDue?: number | string;
  statementId: number;
  lineStatus: number;
  bizSn: string;
  remark?: string;
  addTime?: number;
}

interface StatementRow {
  id: number;
  statementSn: string;
  linkId: number;
  periodStart?: number;
  periodEnd?: number;
  totalDeal?: number | string;
  selfTotal?: number | string;
  peerTotal?: number | string;
  netAmount?: number | string;
  lineCount: number;
  statementType: number;
  selfConfirm: number;
  peerConfirm: number;
  execStatus: number;
  carryAmount?: number | string;
  addTime?: number;
}

const SRC_LABEL: Record<string, string> = {
  xorder: '跨店分账',
  xorder_refund: '跨店退款冲抵',
  supply_return: '供货成交回传',
  supply_refund: '供货退款冲抵',
  prepaid_deduct: '预付扣减',
  carry_in: '结转入',
};

const EXEC_LABEL: Record<number, { label: string; color: 'grey' | 'blue' | 'green' | 'orange' }> = {
  0: { label: '待确认', color: 'grey' },
  1: { label: '已确认待结清', color: 'blue' },
  2: { label: '已结清（登记）', color: 'green' },
  3: { label: '已结转下期', color: 'orange' },
};

const money = (v: any) => (v === undefined || v === null ? '-' : String(v));

export default function HubSettlement() {
  const { t } = useI18n();
  const [tab, setTab] = useState<'accounts' | 'lines' | 'statements'>('accounts');
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((k) => k + 1);
  const [busy, setBusy] = useState(false);
  const [busyId, setBusyId] = useState(0);

  // 开户 / 充值登记
  const [openVisible, setOpenVisible] = useState(false);
  const [openType, setOpenType] = useState('link');
  const [openSubjectId, setOpenSubjectId] = useState<number>(0);
  const [openWarn, setOpenWarn] = useState<number>(0);
  const [adjustRow, setAdjustRow] = useState<AccountRow | null>(null);
  const [adjustAmount, setAdjustAmount] = useState<number>(0);

  // 重算费率
  const [repriceLinkId, setRepriceLinkId] = useState<number>(0);

  // 生成结算单
  const [genVisible, setGenVisible] = useState(false);
  const [genLinkId, setGenLinkId] = useState<number>(0);
  const [genStart, setGenStart] = useState<Date | null>(null);
  const [genEnd, setGenEnd] = useState<Date | null>(null);

  const submitOpen = async () => {
    if (!openSubjectId) {
      Toast.warning(t('请填写主体 ID'));
      return;
    }
    setBusy(true);
    try {
      await apiPost('/hub/settle/accountOpen', { subjectType: openType, subjectId: openSubjectId, warnLine: openWarn });
      Toast.success(t('账户已就绪'));
      setOpenVisible(false);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('开户失败') });
    } finally {
      setBusy(false);
    }
  };

  const submitAdjust = async () => {
    if (!adjustRow || !adjustAmount) {
      Toast.warning(t('请填写登记金额'));
      return;
    }
    setBusy(true);
    try {
      await apiPost('/hub/settle/accountAdjust', { id: adjustRow.id, amount: adjustAmount });
      Toast.success(t('已登记'));
      setAdjustRow(null);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('登记失败') });
    } finally {
      setBusy(false);
    }
  };

  const reprice = async () => {
    if (!repriceLinkId) {
      Toast.warning(t('请填写连接 ID'));
      return;
    }
    setBusy(true);
    try {
      const res = (await apiPost('/hub/settle/reprice', { id: repriceLinkId })) as { repriced?: number };
      Toast.success(`${t('已重算')} ${res.repriced ?? 0} ${t('行')}`);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('重算失败') });
    } finally {
      setBusy(false);
    }
  };

  const toggleLine = async (record: LineRow) => {
    setBusyId(record.id);
    try {
      await apiPost('/hub/settle/setLineStatus', { id: record.id, lineStatus: record.lineStatus === 2 ? 1 : 2 });
      Toast.success(t('行状态已更新'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('更新失败') });
    } finally {
      setBusyId(0);
    }
  };

  const submitGenerate = async () => {
    if (!genLinkId || !genStart || !genEnd) {
      Toast.warning(t('请填写连接 ID 与结算周期'));
      return;
    }
    setBusy(true);
    try {
      const vo = (await apiPost('/hub/settle/generate', {
        linkId: genLinkId,
        periodStart: Math.floor(genStart.getTime() / 1000),
        periodEnd: Math.floor(genEnd.getTime() / 1000),
      })) as StatementRow;
      Toast.success(`${t('结算单已生成')} ${vo.statementSn}`);
      setGenVisible(false);
      setTab('statements');
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('生成失败') });
    } finally {
      setBusy(false);
    }
  };

  const confirmStmt = async (record: StatementRow) => {
    setBusyId(record.id);
    try {
      await apiPost('/hub/settle/confirm', { id: record.id });
      Toast.success(t('已确认'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('确认失败') });
    } finally {
      setBusyId(0);
    }
  };

  const markSettled = async (record: StatementRow) => {
    setBusyId(record.id);
    try {
      await apiPost('/hub/settle/markSettled', { id: record.id });
      Toast.success(t('已结清登记'));
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('登记失败') });
    } finally {
      setBusyId(0);
    }
  };

  const accountColumns: ColumnConfig<AccountRow>[] = [
    { title: 'ID', dataIndex: 'id', width: 70 },
    { title: '主体类型', dataIndex: 'subjectType', width: 110, render: (v: string) => (v === 'link' ? t('连接') : t('凭证')) },
    { title: '主体ID', dataIndex: 'subjectId', width: 100 },
    { title: '余额', dataIndex: 'balance', width: 130, render: money },
    { title: '预警线', dataIndex: 'warnLine', width: 110, render: money },
    {
      title: '告警',
      dataIndex: 'warning',
      width: 110,
      render: (v: boolean) => (v ? <Tag color="red">{t('低于预警线')}</Tag> : <Tag color="green">{t('正常')}</Tag>),
    },
    { title: '更新时间', dataIndex: 'updateTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const lineColumns: ColumnConfig<LineRow>[] = [
    { title: 'ID', dataIndex: 'id', width: 70 },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    {
      title: '来源',
      dataIndex: 'sourceType',
      width: 130,
      render: (v: string) => <Tag color="blue">{t(SRC_LABEL[v] || v)}</Tag>,
    },
    { title: '商品快照', dataIndex: 'itemName', width: 200, render: (v: string) => v || '-' },
    { title: '远端单号', dataIndex: 'remoteOrderSn', width: 170, render: (v: string) => v || '-' },
    { title: '数量', dataIndex: 'quantity', width: 70 },
    { title: '成交额', dataIndex: 'dealAmount', width: 110, render: money },
    { title: '供货额', dataIndex: 'supplyAmount', width: 110, render: money },
    {
      title: '费率',
      dataIndex: 'rate',
      width: 120,
      render: (v: any, record: LineRow) =>
        record.ratePending === 1 ? <Tag color="orange">{t('费率未定')}</Tag> : `${money(v)}%`,
    },
    { title: '中枢应得', dataIndex: 'selfDue', width: 110, render: money },
    { title: '对端应得', dataIndex: 'peerDue', width: 110, render: money },
    { title: '结算单ID', dataIndex: 'statementId', width: 100, render: (v: number) => (v === 0 ? t('未入单') : v) },
    {
      title: '行状态',
      dataIndex: 'lineStatus',
      width: 100,
      render: (v: number) => (v === 2 ? <Tag color="orange">{t('差异挂起')}</Tag> : v === 3 ? <Tag>{t('已冲正')}</Tag> : <Tag color="green">{t('正常')}</Tag>),
    },
    { title: '备注', dataIndex: 'remark', width: 220, render: (v: string) => v || '-' },
    { title: '时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const statementColumns: ColumnConfig<StatementRow>[] = [
    { title: '结算单号', dataIndex: 'statementSn', width: 230, render: (v: string) => v || '-' },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    {
      title: '周期',
      dataIndex: 'periodStart',
      width: 300,
      render: (_: any, record: StatementRow) => `${unixToDateTime(record.periodStart)} ~ ${unixToDateTime(record.periodEnd)}`,
    },
    {
      title: '类型',
      dataIndex: 'statementType',
      width: 90,
      render: (v: number) => (v === 2 ? <Tag color="violet">{t('轧差')}</Tag> : <Tag>{t('普通')}</Tag>),
    },
    { title: '成交总额', dataIndex: 'totalDeal', width: 120, render: money },
    { title: '中枢应得', dataIndex: 'selfTotal', width: 110, render: money },
    { title: '对端应得', dataIndex: 'peerTotal', width: 110, render: money },
    { title: '净额（应付对端）', dataIndex: 'netAmount', width: 140, render: money },
    { title: '行数', dataIndex: 'lineCount', width: 70 },
    {
      title: '双确认',
      dataIndex: 'selfConfirm',
      width: 130,
      render: (_: any, record: StatementRow) => `${record.selfConfirm === 1 ? t('本方✓') : t('本方·')} / ${record.peerConfirm === 1 ? t('对端✓') : t('对端·')}`,
    },
    {
      title: '执行状态',
      dataIndex: 'execStatus',
      width: 130,
      render: (v: number) => {
        const s = EXEC_LABEL[v];
        return s ? <Tag color={s.color}>{t(s.label)}</Tag> : <Tag>{`状态 ${v}`}</Tag>;
      },
    },
    { title: '结转额', dataIndex: 'carryAmount', width: 110, render: money },
    { title: '生成时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <RadioGroup type="button" value={tab} onChange={(e) => setTab(e.target.value as typeof tab)}>
          <Radio value="accounts">{t('预付账户')}</Radio>
          <Radio value="lines">{t('账目行')}</Radio>
          <Radio value="statements">{t('结算单')}</Radio>
        </RadioGroup>
      </div>
      <Typography.Text type="tertiary" style={{ display: 'block', marginBottom: 12 }}>
        {t('账目分账，渠道未执行：本页只记账并生成结算单，不做真实跨站资金划拨；结清为登记动作，实际清分以线下/自有渠道为准。')}
      </Typography.Text>
      {tab === 'accounts' && (
        <ListPage<AccountRow>
          key={`accounts-${listKey}`}
          api="/hub/settle/accounts"
          rowKey="id"
          pageSize={10}
          actionWidth={110}
          columns={accountColumns}
          toolbarExtra={
            <Button theme="solid" type="primary" onClick={() => setOpenVisible(true)}>
              {t('开户')}
            </Button>
          }
          rowActions={(record) => (
            <Button theme="borderless" size="small" type="primary" onClick={() => { setAdjustRow(record); setAdjustAmount(0); }}>
              {t('充值登记')}
            </Button>
          )}
        />
      )}
      {tab === 'lines' && (
        <ListPage<LineRow>
          key={`lines-${listKey}`}
          api="/hub/settle/lines"
          rowKey="id"
          pageSize={10}
          actionWidth={100}
          filters={[
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 120 },
            {
              field: 'sourceType',
              label: '来源',
              type: 'select',
              placeholder: '全部来源',
              width: 160,
              options: Object.entries(SRC_LABEL).map(([value, label]) => ({ label, value })),
            },
            {
              field: 'lineStatus',
              label: '行状态',
              type: 'select',
              placeholder: '全部状态',
              width: 140,
              options: [
                { label: '正常', value: 1 },
                { label: '差异挂起', value: 2 },
                { label: '已冲正', value: 3 },
              ],
            },
          ]}
          columns={lineColumns}
          toolbarExtra={
            <Space>
              <InputNumber placeholder={t('连接 ID')} value={repriceLinkId || undefined} onChange={(v) => setRepriceLinkId(Number(v || 0))} style={{ width: 120 }} />
              <Button loading={busy} onClick={() => void reprice()}>
                {t('重算未定费率')}
              </Button>
            </Space>
          }
          rowActions={(record) =>
            record.statementId === 0 ? (
              <Button theme="borderless" size="small" type="warning" loading={busyId === record.id} onClick={() => void toggleLine(record)}>
                {record.lineStatus === 2 ? t('恢复') : t('差异挂起')}
              </Button>
            ) : null
          }
        />
      )}
      {tab === 'statements' && (
        <ListPage<StatementRow>
          key={`statements-${listKey}`}
          api="/hub/settle/statements"
          rowKey="id"
          pageSize={10}
          actionWidth={170}
          filters={[
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 120 },
            {
              field: 'execStatus',
              label: '执行状态',
              type: 'select',
              placeholder: '全部状态',
              width: 150,
              options: Object.entries(EXEC_LABEL).map(([value, s]) => ({ label: s.label, value: Number(value) })),
            },
          ]}
          columns={statementColumns}
          toolbarExtra={
            <Button theme="solid" type="primary" onClick={() => setGenVisible(true)}>
              {t('生成结算单')}
            </Button>
          }
          rowActions={(record) => (
            <Space spacing={2}>
              {record.execStatus === 0 && (
                <Button theme="borderless" size="small" type="primary" loading={busyId === record.id} onClick={() => void confirmStmt(record)}>
                  {t('本方确认')}
                </Button>
              )}
              {record.execStatus === 1 && (
                <Button theme="borderless" size="small" type="primary" loading={busyId === record.id} onClick={() => void markSettled(record)}>
                  {t('结清登记')}
                </Button>
              )}
            </Space>
          )}
        />
      )}
      <Modal
        title={t('预付账户开户')}
        visible={openVisible}
        onCancel={() => setOpenVisible(false)}
        onOk={() => void submitOpen()}
        okText={t('开户')}
        cancelText={t('取消')}
        confirmLoading={busy}
        width={440}
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <Select
            value={openType}
            onChange={(v) => setOpenType(String(v))}
            style={{ width: '100%' }}
            optionList={[
              { label: t('连接'), value: 'link' },
              { label: t('凭证'), value: 'credential' },
            ]}
          />
          <InputNumber placeholder={t('主体 ID（连接 ID 或凭证 ID）')} value={openSubjectId || undefined} onChange={(v) => setOpenSubjectId(Number(v || 0))} style={{ width: '100%' }} />
          <InputNumber placeholder={t('预警线（余额低于此值告警）')} value={openWarn || undefined} onChange={(v) => setOpenWarn(Number(v || 0))} style={{ width: '100%' }} />
        </div>
      </Modal>
      <Modal
        title={t('充值登记（只记账）')}
        visible={adjustRow !== null}
        onCancel={() => setAdjustRow(null)}
        onOk={() => void submitAdjust()}
        okText={t('登记')}
        cancelText={t('取消')}
        confirmLoading={busy}
        width={440}
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <Typography.Text>
            {t('账户')} #{adjustRow?.id} · {t('当前余额')} {money(adjustRow?.balance)}
          </Typography.Text>
          <InputNumber placeholder={t('登记金额（正为充值，负为冲减）')} value={adjustAmount || undefined} onChange={(v) => setAdjustAmount(Number(v || 0))} style={{ width: '100%' }} />
        </div>
      </Modal>
      <Modal
        title={t('生成结算单')}
        visible={genVisible}
        onCancel={() => setGenVisible(false)}
        onOk={() => void submitGenerate()}
        okText={t('生成')}
        cancelText={t('取消')}
        confirmLoading={busy}
        width={440}
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <InputNumber placeholder={t('连接 ID')} value={genLinkId || undefined} onChange={(v) => setGenLinkId(Number(v || 0))} style={{ width: '100%' }} />
          <DatePicker type="dateTime" placeholder={t('周期开始')} value={genStart ?? undefined} onChange={(d) => setGenStart(d ? new Date(d as any) : null)} style={{ width: '100%' }} />
          <DatePicker type="dateTime" placeholder={t('周期结束')} value={genEnd ?? undefined} onChange={(d) => setGenEnd(d ? new Date(d as any) : null)} style={{ width: '100%' }} />
          <Typography.Text type="tertiary">{t('周期内存在未定费率行时生成会被拦截，请先在连接上设置覆盖费率并在账目行页重算。')}</Typography.Text>
        </div>
      </Modal>
    </div>
  );
}
