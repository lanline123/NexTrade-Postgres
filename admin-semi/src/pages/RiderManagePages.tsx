import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Input,
  Modal,
  Radio,
  RadioGroup,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  TextArea,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text } = Typography;

/** 骑手 MVP（我方新增页面；地基接口为原版有·我方补齐接线）：
 *  - StaffApplyManage：入驻审核（会员申请 → 审核 → 生成 service_staff）
 *  - DeliveryOrderManage：派送单管理（指派/改派/取消，人员选择走 availableStaff）
 */

function fmtTs(ts?: number): string {
  if (!ts) return '-';
  const d = new Date(ts * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

interface ApplyRow {
  applyId: number;
  memberId: number;
  shopId: number;
  shopTitle?: string;
  type: number;
  name: string;
  phone: string;
  idCard?: string;
  remark?: string;
  status: number;
  auditRemark?: string;
  staffId?: number;
  createTime?: number;
}

const APPLY_STATUS: Record<number, { text: string; color: 'amber' | 'green' | 'red' | 'grey' }> = {
  0: { text: '待审核', color: 'amber' },
  1: { text: '已通过', color: 'green' },
  2: { text: '已驳回', color: 'red' },
};

export function StaffApplyManage() {
  const { t } = useI18n();
  const [tab, setTab] = useState<string>('0');
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<ApplyRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [auditRow, setAuditRow] = useState<ApplyRow | null>(null);
  const [auditPass, setAuditPass] = useState(true);
  const [auditRemark, setAuditRemark] = useState('');
  const [auditing, setAuditing] = useState(false);
  const size = 20;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiGet<{ records: ApplyRow[]; total: number }>('/o2o/serviceStaffApply/list', {
        status: tab === 'all' ? -1 : Number(tab),
        keyword,
        page,
        size,
      });
      setRows(res.records || []);
      setTotal(res.total || 0);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
    }
  }, [tab, keyword, page, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const openAudit = (row: ApplyRow, pass: boolean) => {
    setAuditRow(row);
    setAuditPass(pass);
    setAuditRemark('');
  };

  const submitAudit = async () => {
    if (!auditRow) return;
    setAuditing(true);
    try {
      await apiPost('/o2o/serviceStaffApply/audit', {
        id: auditRow.applyId,
        pass: auditPass,
        remark: auditRemark,
      });
      Toast.success({ content: t(auditPass ? '已通过，服务人员已生成' : '已驳回') });
      setAuditRow(null);
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('审核失败') });
    } finally {
      setAuditing(false);
    }
  };

  const columns = [
    { title: t('申请ID'), dataIndex: 'applyId', width: 80 },
    {
      title: t('申请人'),
      width: 170,
      render: (_: unknown, r: ApplyRow) => (
        <div>
          <div>{r.name}</div>
          <Text type="tertiary" size="small">{r.phone}</Text>
        </div>
      ),
    },
    {
      title: t('类型'),
      width: 90,
      render: (_: unknown, r: ApplyRow) => (r.type === 2 ? t('服务员') : t('配送员')),
    },
    {
      title: t('入驻店铺'),
      width: 180,
      render: (_: unknown, r: ApplyRow) => `${r.shopTitle || '-'}（#${r.shopId}）`,
    },
    { title: t('会员ID'), dataIndex: 'memberId', width: 90 },
    {
      title: t('备注'),
      width: 160,
      render: (_: unknown, r: ApplyRow) => r.remark || '-',
    },
    {
      title: t('状态'),
      width: 100,
      render: (_: unknown, r: ApplyRow) => {
        const st = APPLY_STATUS[r.status] || { text: String(r.status), color: 'grey' as const };
        return <Tag color={st.color}>{t(st.text)}</Tag>;
      },
    },
    {
      title: t('申请时间'),
      width: 150,
      render: (_: unknown, r: ApplyRow) => fmtTs(r.createTime),
    },
    {
      title: t('操作'),
      width: 150,
      render: (_: unknown, r: ApplyRow) =>
        r.status === 0 ? (
          <Space>
            <Button size="small" theme="solid" type="primary" onClick={() => openAudit(r, true)}>
              {t('通过')}
            </Button>
            <Button size="small" type="danger" onClick={() => openAudit(r, false)}>
              {t('驳回')}
            </Button>
          </Space>
        ) : (
          <Text type="tertiary" size="small">{r.auditRemark || '-'}</Text>
        ),
    },
  ];

  return (
    <div>
      <Tabs
        type="line"
        activeKey={tab}
        onChange={(k) => {
          setTab(k);
          setPage(1);
        }}
      >
        <Tabs.TabPane tab={t('待审核')} itemKey="0" />
        <Tabs.TabPane tab={t('已通过')} itemKey="1" />
        <Tabs.TabPane tab={t('已驳回')} itemKey="2" />
        <Tabs.TabPane tab={t('全部')} itemKey="all" />
      </Tabs>
      <div style={{ margin: '12px 0' }}>
        <Input
          style={{ width: 260 }}
          placeholder={t('姓名 / 手机号')}
          value={keyword}
          onChange={(v) => {
            setKeyword(v);
            setPage(1);
          }}
          showClear
        />
      </div>
      <Table
        columns={columns}
        dataSource={rows}
        rowKey="applyId"
        loading={loading}
        pagination={{ currentPage: page, pageSize: size, total, onPageChange: setPage }}
      />
      <Modal
        title={t(auditPass ? '通过入驻申请' : '驳回入驻申请')}
        visible={!!auditRow}
        onCancel={() => setAuditRow(null)}
        onOk={submitAudit}
        confirmLoading={auditing}
        okText={t(auditPass ? '确认通过' : '确认驳回')}
        cancelText={t('取消')}
      >
        {auditRow && (
          <div>
            <p>
              {auditRow.name}（{auditRow.phone}）申请成为「{auditRow.shopTitle || `#${auditRow.shopId}`}」的
              {auditRow.type === 2 ? t('服务员') : t('配送员')}
              {auditPass ? t('，通过后将生成服务人员记录，可立即接单。') : t('，请填写驳回原因。')}
            </p>
            <TextArea
              placeholder={t(auditPass ? '审核备注（选填）' : '驳回原因')}
              value={auditRemark}
              onChange={setAuditRemark}
              rows={3}
            />
          </div>
        )}
      </Modal>
    </div>
  );
}

/* ================================================================== */

interface DeliveryRow {
  deliveryOrderId: number;
  deliveryOrderSn: string;
  orderSn: string;
  shopId: number;
  type: number;
  dispatchMode: number;
  status: number;
  staffId?: number | null;
  staffName?: string;
  consignee: string;
  mobile: string;
  address?: string;
  orderAmount?: number;
  serviceFee?: number;
  createTime?: number;
  cancelTime?: number;
  cancelReason?: string;
  cancelBy?: number | null;
}

/** 取消方口径与后端 delivery_order.cancel_by 一致：1-配送员/服务员，2-后台管理员，3-系统。 */
const CANCEL_BY_TEXT: Record<number, string> = {
  1: '配送员取消',
  2: '后台取消',
  3: '系统取消',
};

interface StaffOption {
  id: number;
  name: string;
  phone: string;
  workStatus: number;
  completedOrderCount?: number;
}

const DELIVERY_STATUS: Record<number, { text: string; color: 'amber' | 'blue' | 'violet' | 'green' | 'red' | 'grey' }> = {
  10: { text: '待派发', color: 'amber' },
  20: { text: '待取货', color: 'blue' },
  30: { text: '配送中', color: 'violet' },
  40: { text: '已完成', color: 'green' },
  50: { text: '已取消', color: 'red' },
};

export function DeliveryOrderManage() {
  const { t } = useI18n();
  const [tab, setTab] = useState<string>('all');
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<DeliveryRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [assignRow, setAssignRow] = useState<DeliveryRow | null>(null);
  const [staffOptions, setStaffOptions] = useState<StaffOption[]>([]);
  const [staffLoading, setStaffLoading] = useState(false);
  const [pickStaff, setPickStaff] = useState<number | undefined>(undefined);
  const [assigning, setAssigning] = useState(false);
  const size = 20;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiGet<{ records: DeliveryRow[]; total: number }>('/o2o/deliveryOrder/list', {
        status: tab === 'all' ? -1 : Number(tab),
        keyword,
        page,
        size,
      });
      setRows(res.records || []);
      setTotal(res.total || 0);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
    }
  }, [tab, keyword, page, t]);

  useEffect(() => {
    void load();
  }, [load]);

  const openAssign = async (row: DeliveryRow) => {
    setAssignRow(row);
    setPickStaff(undefined);
    setStaffOptions([]);
    setStaffLoading(true);
    try {
      const res = await apiGet<{ records: StaffOption[] }>('/o2o/dispatch/availableStaff', {
        deliveryOrderId: row.deliveryOrderId,
        type: row.type,
      });
      setStaffOptions(res.records || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载可指派人员失败') });
    } finally {
      setStaffLoading(false);
    }
  };

  const submitAssign = async () => {
    if (!assignRow || !pickStaff) {
      Toast.warning({ content: t('请选择指派人员') });
      return;
    }
    setAssigning(true);
    try {
      await apiPost('/o2o/deliveryOrder/dispatch', {
        deliveryOrderId: assignRow.deliveryOrderId,
        staffId: pickStaff,
      });
      Toast.success({ content: t('指派成功') });
      setAssignRow(null);
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('指派失败') });
    } finally {
      setAssigning(false);
    }
  };

  const cancelOrder = (row: DeliveryRow) => {
    Modal.confirm({
      title: t('取消派送单'),
      content: t('确认取消该派送单？取消后不可恢复。'),
      okText: t('确认取消'),
      cancelText: t('返回'),
      onOk: async () => {
        try {
          await apiPost('/o2o/deliveryOrder/cancel', {
            deliveryOrderId: row.deliveryOrderId,
            cancelReason: '后台取消',
          });
          Toast.success({ content: t('已取消') });
          void load();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('取消失败') });
        }
      },
    });
  };

  const columns = [
    { title: t('派送单号'), dataIndex: 'deliveryOrderSn', width: 130 },
    {
      title: t('订单 / 收货人'),
      width: 200,
      render: (_: unknown, r: DeliveryRow) => (
        <div>
          <div>{r.orderSn}</div>
          <Text type="tertiary" size="small">
            {r.consignee} {r.mobile}
          </Text>
        </div>
      ),
    },
    {
      title: t('类型'),
      width: 80,
      render: (_: unknown, r: DeliveryRow) => (r.type === 2 ? t('上门服务') : t('同城配送')),
    },
    {
      title: t('状态'),
      width: 100,
      render: (_: unknown, r: DeliveryRow) => {
        const st = DELIVERY_STATUS[r.status] || { text: String(r.status), color: 'grey' as const };
        return <Tag color={st.color}>{t(st.text)}</Tag>;
      },
    },
    {
      title: t('执行人员'),
      width: 130,
      render: (_: unknown, r: DeliveryRow) =>
        r.staffName ? (
          <span>
            {r.staffName}
            <Text type="tertiary" size="small">（{r.dispatchMode === 1 ? t('抢单') : t('指派')}）</Text>
          </span>
        ) : (
          <Text type="tertiary">{t('未分配')}</Text>
        ),
    },
    {
      title: t('取消信息'),
      width: 190,
      render: (_: unknown, r: DeliveryRow) =>
        r.status === 50 ? (
          <div>
            <div>{t(CANCEL_BY_TEXT[r.cancelBy || 3] || '系统取消')}</div>
            <Text type="tertiary" size="small">
              {r.cancelReason || '-'} · {fmtTs(r.cancelTime)}
            </Text>
          </div>
        ) : (
          <Text type="tertiary">-</Text>
        ),
    },
    {
      title: t('订单金额'),
      width: 100,
      render: (_: unknown, r: DeliveryRow) => `¥${Number(r.orderAmount || 0).toFixed(2)}`,
    },
    {
      title: t('创建时间'),
      width: 150,
      render: (_: unknown, r: DeliveryRow) => fmtTs(r.createTime),
    },
    {
      title: t('操作'),
      width: 170,
      render: (_: unknown, r: DeliveryRow) => (
        <Space>
          {[10, 20, 30].includes(r.status) && (
            <Button size="small" theme="solid" type="primary" onClick={() => openAssign(r)}>
              {t(r.staffId ? '改派' : '指派')}
            </Button>
          )}
          {[10, 20, 30].includes(r.status) && (
            <Button size="small" type="danger" onClick={() => cancelOrder(r)}>
              {t('取消')}
            </Button>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Tabs
        type="line"
        activeKey={tab}
        onChange={(k) => {
          setTab(k);
          setPage(1);
        }}
      >
        <Tabs.TabPane tab={t('全部')} itemKey="all" />
        <Tabs.TabPane tab={t('待派发')} itemKey="10" />
        <Tabs.TabPane tab={t('待取货')} itemKey="20" />
        <Tabs.TabPane tab={t('配送中')} itemKey="30" />
        <Tabs.TabPane tab={t('已完成')} itemKey="40" />
        <Tabs.TabPane tab={t('已取消')} itemKey="50" />
      </Tabs>
      <div style={{ margin: '12px 0' }}>
        <Input
          style={{ width: 260 }}
          placeholder={t('派送单号 / 订单号 / 收货人')}
          value={keyword}
          onChange={(v) => {
            setKeyword(v);
            setPage(1);
          }}
          showClear
        />
      </div>
      <Table
        columns={columns}
        dataSource={rows}
        rowKey="deliveryOrderId"
        loading={loading}
        pagination={{ currentPage: page, pageSize: size, total, onPageChange: setPage }}
      />
      <Modal
        title={t(assignRow?.staffId ? '改派' : '指派')}
        visible={!!assignRow}
        onCancel={() => setAssignRow(null)}
        onOk={submitAssign}
        confirmLoading={assigning}
        okText={t('确认指派')}
        cancelText={t('取消')}
      >
        {assignRow && (
          <div>
            <p>
              {t('派送单')} {assignRow.deliveryOrderSn}（{assignRow.type === 2 ? t('上门服务') : t('同城配送')}，
              {t('店铺')} #{assignRow.shopId}）
            </p>
            <RadioGroup
              value={pickStaff}
              onChange={(e) => setPickStaff(Number(e.target.value))}
              direction="vertical"
            >
              {staffOptions.map((staff) => (
                <Radio key={staff.id} value={staff.id}>
                  {staff.name}（{staff.phone}）
                  <Text type="tertiary" size="small">
                    {' '}
                    {staff.workStatus === 2 ? t('忙碌中') : staff.workStatus === 1 ? t('空闲') : t('已下班')} ·{' '}
                    {t('已完成 {count} 单', { count: staff.completedOrderCount || 0 })}
                  </Text>
                </Radio>
              ))}
            </RadioGroup>
            {!staffLoading && staffOptions.length === 0 && (
              <Text type="tertiary">{t('该店铺暂无在职的同类型人员')}</Text>
            )}
            {staffLoading && <Text type="tertiary">{t('加载中…')}</Text>}
            <div style={{ marginTop: 8 }}>
              <Select
                style={{ width: '100%' }}
                placeholder={t('或搜索选择人员')}
                value={pickStaff}
                onChange={(v) => setPickStaff(typeof v === 'number' ? v : undefined)}
                optionList={staffOptions.map((staff) => ({
                  label: `${staff.name}（${staff.phone}）`,
                  value: staff.id,
                }))}
              />
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
