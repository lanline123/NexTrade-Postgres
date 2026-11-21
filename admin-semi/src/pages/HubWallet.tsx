// 我方新增（万象互联三期④）：中央账本页（万象互联 → 中央账本）。
// 中枢侧：联邦账户与账本行（只记账不划拨；差错只冲正不改史）+ 手工记账
// （积分发放/余额入账/冲正）+ 过期冻结释放；成员侧：本机镜像 + 存量并账。
// 并账后本地余额支付停用（后端守卫）；本地余额列不清零，运营按快照核对后手工处理。
import { useState } from 'react';
import { Button, Input, InputNumber, Modal, Radio, RadioGroup, Select, Space, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import { unixToDateTime } from '../utils/format';

interface AccountRow {
  globalMemberId: string;
  pointsBalance: number;
  moneyBalance?: number | string;
  frozenMoney?: number | string;
  version: number;
  status: number;
}

interface LineRow {
  id: number;
  globalMemberId: string;
  linkId: number;
  currency: string;
  bizType: string;
  amount?: number | string;
  balanceAfter?: number | string;
  frozenAfter?: number | string;
  bizSn: string;
  refBizSn?: string;
  status: number;
  remark?: string;
  addTime?: number;
}

interface MirrorRow {
  id: number;
  linkId: number;
  localUserId: number;
  globalMemberId: string;
  bizSn: string;
  currency: string;
  bizType: string;
  amount?: number | string;
  balanceAfter?: number | string;
  addTime?: number;
}

const BIZ_LABEL: Record<string, string> = {
  points_grant: '积分发放',
  points_spend: '积分消费',
  money_credit: '余额入账',
  money_debit: '余额扣减',
  money_freeze: '余额冻结',
  money_confirm: '冻结确认',
  money_release: '冻结释放',
  reverse: '冲正',
  merge_in: '存量并账',
};

export default function HubWallet() {
  const { t } = useI18n();
  const [tab, setTab] = useState<'accounts' | 'lines' | 'mirror'>('accounts');
  const [listKey, setListKey] = useState(0);
  const refresh = () => setListKey((k) => k + 1);
  const [busy, setBusy] = useState(false);

  // 手工记账表单
  const [postVisible, setPostVisible] = useState(false);
  const [postGid, setPostGid] = useState('');
  const [postBiz, setPostBiz] = useState('points_grant');
  const [postCurrency, setPostCurrency] = useState('points');
  const [postAmount, setPostAmount] = useState<number>(0);
  const [postRef, setPostRef] = useState('');
  const [postRemark, setPostRemark] = useState('');

  // 并账
  const [mergeLinkId, setMergeLinkId] = useState<number>(0);

  const submitPost = async () => {
    if (!postGid.trim()) {
      Toast.warning(t('请填写会员全局 ID'));
      return;
    }
    setBusy(true);
    try {
      await apiPost('/hub/wallet/postLine', {
        globalMemberId: postGid.trim(),
        bizType: postBiz,
        currency: postCurrency,
        amount: postAmount,
        refBizSn: postRef.trim(),
        remark: postRemark.trim(),
      });
      Toast.success(t('已记账'));
      setPostVisible(false);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('记账失败') });
    } finally {
      setBusy(false);
    }
  };

  const releaseExpired = async () => {
    setBusy(true);
    try {
      const res = (await apiPost('/hub/wallet/releaseExpired', {})) as { released?: number; skipped?: number };
      Toast.success(`${t('已释放')} ${res.released ?? 0} · ${t('跳过')} ${res.skipped ?? 0}`);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('释放失败') });
    } finally {
      setBusy(false);
    }
  };

  const merge = async () => {
    if (!mergeLinkId) {
      Toast.warning(t('请填写连接 ID'));
      return;
    }
    setBusy(true);
    try {
      const res = (await apiPost('/hub/myLinks/mergeWallet', { id: mergeLinkId })) as { merged?: number; skipped?: number };
      Toast.success(`${t('已并账')} ${res.merged ?? 0} · ${t('跳过')} ${res.skipped ?? 0}`);
      refresh();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('并账失败') });
    } finally {
      setBusy(false);
    }
  };

  const accountColumns: ColumnConfig<AccountRow>[] = [
    { title: '会员全局ID', dataIndex: 'globalMemberId', width: 280, render: (v: string) => v || '-' },
    { title: '积分余额', dataIndex: 'pointsBalance', width: 120 },
    { title: '余额', dataIndex: 'moneyBalance', width: 120, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '冻结余额', dataIndex: 'frozenMoney', width: 120, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '版本', dataIndex: 'version', width: 80 },
    {
      title: '状态',
      dataIndex: 'status',
      width: 100,
      render: (v: number) => (v === 1 ? <Tag color="green">{t('正常')}</Tag> : <Tag color="red">{t('冻结停用')}</Tag>),
    },
  ];

  const lineColumns: ColumnConfig<LineRow>[] = [
    { title: 'ID', dataIndex: 'id', width: 70 },
    { title: '会员全局ID', dataIndex: 'globalMemberId', width: 240, render: (v: string) => v || '-' },
    { title: '来源连接', dataIndex: 'linkId', width: 90, render: (v: number) => (v === 0 ? t('中枢管理') : v) },
    {
      title: '业务',
      dataIndex: 'bizType',
      width: 110,
      render: (v: string) => <Tag color="blue">{t(BIZ_LABEL[v] || v)}</Tag>,
    },
    { title: '币种', dataIndex: 'currency', width: 80, render: (v: string) => (v === 'points' ? t('积分') : t('余额')) },
    { title: '金额', dataIndex: 'amount', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '记账后余额', dataIndex: 'balanceAfter', width: 120, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '业务单号', dataIndex: 'bizSn', width: 220, render: (v: string) => v || '-' },
    { title: '引用单号', dataIndex: 'refBizSn', width: 180, render: (v: string) => v || '-' },
    {
      title: '状态',
      dataIndex: 'status',
      width: 90,
      render: (v: number) => (v === 2 ? <Tag color="orange">{t('已冲正')}</Tag> : <Tag color="green">{t('已记账')}</Tag>),
    },
    { title: '备注', dataIndex: 'remark', width: 200, render: (v: string) => v || '-' },
    { title: '时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  const mirrorColumns: ColumnConfig<MirrorRow>[] = [
    { title: 'ID', dataIndex: 'id', width: 70 },
    { title: '连接ID', dataIndex: 'linkId', width: 90 },
    { title: '本地会员ID', dataIndex: 'localUserId', width: 110 },
    { title: '会员全局ID', dataIndex: 'globalMemberId', width: 240, render: (v: string) => v || '-' },
    {
      title: '业务',
      dataIndex: 'bizType',
      width: 110,
      render: (v: string) => <Tag color="blue">{t(BIZ_LABEL[v] || v)}</Tag>,
    },
    { title: '币种', dataIndex: 'currency', width: 80, render: (v: string) => (v === 'points' ? t('积分') : t('余额')) },
    { title: '金额', dataIndex: 'amount', width: 110, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '中枢记账后余额', dataIndex: 'balanceAfter', width: 140, render: (v: any) => (v === undefined || v === null ? '-' : String(v)) },
    { title: '业务单号', dataIndex: 'bizSn', width: 220, render: (v: string) => v || '-' },
    { title: '时间', dataIndex: 'addTime', width: 170, render: (v: number) => unixToDateTime(v) },
  ];

  return (
    <div>
      <div style={{ marginBottom: 12 }}>
        <RadioGroup type="button" value={tab} onChange={(e) => setTab(e.target.value as typeof tab)}>
          <Radio value="accounts">{t('联邦账户（中枢）')}</Radio>
          <Radio value="lines">{t('账本行（中枢）')}</Radio>
          <Radio value="mirror">{t('本机镜像与并账')}</Radio>
        </RadioGroup>
      </div>
      <Typography.Text type="tertiary" style={{ display: 'block', marginBottom: 12 }}>
        {t('中央账本只记账、不划拨真实资金；差错只能冲正，不能改史。并账后会员本地余额支付停用，本地余额列不清零——运营按并账快照核对后手工处理。')}
      </Typography.Text>
      {tab === 'accounts' && (
        <ListPage<AccountRow>
          key={`accounts-${listKey}`}
          api="/hub/wallet/accounts"
          rowKey="globalMemberId"
          pageSize={10}
          filters={[{ field: 'keyword', label: '全局ID', type: 'input', placeholder: '会员全局 ID', width: 260 }]}
          columns={accountColumns}
          toolbarExtra={
            <Space>
              <Button theme="solid" type="primary" onClick={() => setPostVisible(true)}>
                {t('手工记账')}
              </Button>
              <Button loading={busy} onClick={() => void releaseExpired()}>
                {t('释放过期冻结')}
              </Button>
            </Space>
          }
        />
      )}
      {tab === 'lines' && (
        <ListPage<LineRow>
          key={`lines-${listKey}`}
          api="/hub/wallet/lines"
          rowKey="id"
          pageSize={10}
          filters={[
            { field: 'globalMemberId', label: '全局ID', type: 'input', placeholder: '会员全局 ID', width: 240 },
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 120 },
            {
              field: 'bizType',
              label: '业务',
              type: 'select',
              placeholder: '全部业务',
              width: 150,
              options: Object.entries(BIZ_LABEL).map(([value, label]) => ({ label, value })),
            },
            {
              field: 'currency',
              label: '币种',
              type: 'select',
              placeholder: '全部币种',
              width: 120,
              options: [
                { label: '积分', value: 'points' },
                { label: '余额', value: 'money' },
              ],
            },
          ]}
          columns={lineColumns}
        />
      )}
      {tab === 'mirror' && (
        <ListPage<MirrorRow>
          key={`mirror-${listKey}`}
          api="/hub/wallet/mirror"
          rowKey="id"
          pageSize={10}
          filters={[
            { field: 'localUserId', label: '本地会员ID', type: 'number', placeholder: '本地会员 ID', width: 140 },
            { field: 'linkId', label: '连接ID', type: 'number', placeholder: '连接 ID', width: 120 },
          ]}
          columns={mirrorColumns}
          toolbarExtra={
            <Space>
              <InputNumber placeholder={t('连接 ID')} value={mergeLinkId || undefined} onChange={(v) => setMergeLinkId(Number(v || 0))} style={{ width: 130 }} />
              <Button theme="solid" type="primary" loading={busy} onClick={() => void merge()}>
                {t('执行并账')}
              </Button>
            </Space>
          }
        />
      )}
      <Modal
        title={t('手工记账（中枢）')}
        visible={postVisible}
        onCancel={() => setPostVisible(false)}
        onOk={() => void submitPost()}
        okText={t('记账')}
        cancelText={t('取消')}
        confirmLoading={busy}
        width={480}
      >
        <div style={{ display: 'grid', gap: 12 }}>
          <Input placeholder={t('会员全局 ID')} value={postGid} onChange={setPostGid} />
          <Select
            value={postBiz}
            onChange={(v) => {
              const biz = String(v);
              setPostBiz(biz);
              setPostCurrency(biz === 'points_grant' ? 'points' : 'money');
            }}
            style={{ width: '100%' }}
            optionList={[
              { label: t('积分发放'), value: 'points_grant' },
              { label: t('余额入账'), value: 'money_credit' },
              { label: t('冲正'), value: 'reverse' },
            ]}
          />
          <Select
            value={postCurrency}
            onChange={(v) => setPostCurrency(String(v))}
            style={{ width: '100%' }}
            optionList={[
              { label: t('积分'), value: 'points' },
              { label: t('余额'), value: 'money' },
            ]}
          />
          <InputNumber placeholder={t('金额（正数）')} value={postAmount || undefined} onChange={(v) => setPostAmount(Number(v || 0))} style={{ width: '100%' }} min={0} />
          <Input placeholder={t('引用业务单号（冲正时填原行 bizSn）')} value={postRef} onChange={setPostRef} />
          <Input placeholder={t('备注')} value={postRemark} onChange={setPostRemark} />
        </div>
      </Modal>
    </div>
  );
}
