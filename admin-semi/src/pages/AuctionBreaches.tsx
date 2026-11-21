import { useState } from 'react';
import { Button, Input, InputNumber, Modal, Select, Space, Toast, Typography } from '@douyinfe/semi-ui';
import { apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';

const { Text, Title } = Typography;

interface BreachRow {
  breachId: number;
  username?: string;
  nickname?: string;
  userId?: number;
  itemTitle?: string;
  orderId?: number;
  depositAmount?: string | number;
  penaltyAmount?: string | number;
  breachTimeText?: string;
  waived?: number;
  waiveReason?: string;
  breachCount?: number;
}

interface BanRow {
  banId: number;
  username?: string;
  nickname?: string;
  userId?: number;
  banTypeName?: string;
  endTimeText?: string;
  reason?: string;
  status?: number;
  addTimeText?: string;
}

/** 拍卖违约处理（商品 → 拍卖 → 违约处理）：违约记录 + 扣罚改判 + 参拍封禁管理。 */
export default function AuctionBreaches() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [waiveRow, setWaiveRow] = useState<BreachRow | null>(null);
  const [waiveReason, setWaiveReason] = useState('');
  const [banOpen, setBanOpen] = useState(false);
  const [banUserId, setBanUserId] = useState<number | undefined>(undefined);
  const [banType, setBanType] = useState<number>(1);
  const [banDays, setBanDays] = useState<number>(30);
  const [banReason, setBanReason] = useState('');

  const reload = () => setReloadKey((key) => key + 1);

  const submitWaive = async () => {
    if (!waiveRow) return;
    if (!waiveReason.trim()) {
      Toast.error({ content: t('改判必须填写理由') });
      return;
    }
    try {
      await apiPost('/promotion/auction/breach/waivePenalty', { breachId: waiveRow.breachId, reason: waiveReason.trim() });
      Toast.success({ content: t('已改判，扣罚金额已退回会员余额') });
      setWaiveRow(null);
      setWaiveReason('');
      reload();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const submitBan = async () => {
    if (!banUserId) {
      Toast.error({ content: t('请填写会员ID') });
      return;
    }
    try {
      await apiPost('/promotion/auction/ban/save', {
        userId: banUserId,
        banType,
        days: banType === 1 ? banDays : undefined,
        reason: banReason,
      });
      Toast.success({ content: t('封禁成功') });
      setBanOpen(false);
      reload();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const unban = (row: BanRow) => {
    Modal.confirm({
      title: t('确认解除该会员的参拍封禁吗？'),
      onOk: async () => {
        try {
          await apiPost('/promotion/auction/ban/unban', { banId: row.banId });
          Toast.success({ content: t('已解封') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const breachColumns: ColumnConfig<BreachRow>[] = [
    {
      title: t('会员'),
      dataIndex: 'username',
      width: 150,
      render: (_: string, row: BreachRow) => `${row.nickname || row.username || '-'}（#${row.userId ?? '-'}）`,
    },
    { title: t('拍品'), dataIndex: 'itemTitle', width: 220, render: (v: string) => v || '-' },
    { title: t('订单'), dataIndex: 'orderId', width: 90, render: (v: any) => (v ? `#${v}` : '-') },
    { title: t('保证金'), dataIndex: 'depositAmount', width: 100, render: (v: any) => `¥${v ?? '-'}` },
    { title: t('扣罚'), dataIndex: 'penaltyAmount', width: 100, render: (v: any) => `¥${v ?? '-'}` },
    { title: t('累计违约'), dataIndex: 'breachCount', width: 90 },
    { title: t('违约时间'), dataIndex: 'breachTimeText', width: 160 },
    {
      title: t('改判'),
      dataIndex: 'waived',
      width: 200,
      render: (v: number, row: BreachRow) =>
        v === 1 ? <Text type="tertiary">{t('已豁免')}：{row.waiveReason || ''}</Text> : '-',
    },
  ];

  const banColumns: ColumnConfig<BanRow>[] = [
    {
      title: t('会员'),
      dataIndex: 'username',
      width: 150,
      render: (_: string, row: BanRow) => `${row.nickname || row.username || '-'}（#${row.userId ?? '-'}）`,
    },
    { title: t('类型'), dataIndex: 'banTypeName', width: 90, render: (v: string) => (v ? t(v) : '-') },
    { title: t('封禁至'), dataIndex: 'endTimeText', width: 160 },
    { title: t('原因'), dataIndex: 'reason', width: 260, render: (v: string) => v || '-' },
    {
      title: t('状态'),
      dataIndex: 'status',
      width: 90,
      render: (v: number) => (v === 1 ? t('生效中') : t('已解除')),
    },
    { title: t('创建时间'), dataIndex: 'addTimeText', width: 160 },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Title heading={5} style={{ marginBottom: 8 }}>
        {t('违约记录')}
      </Title>
      <ListPage<BreachRow>
        key={`b-${reloadKey}`}
        api="/promotion/auction/breach/list"
        rowKey="breachId"
        pageSize={10}
        actionWidth={110}
        filters={[{ field: 'userId', label: t('会员ID'), type: 'number', width: 140 }]}
        columns={breachColumns}
        rowActions={(row) =>
          row.waived !== 1 ? (
            <Button theme="borderless" size="small" onClick={() => setWaiveRow(row)}>
              {t('改判豁免')}
            </Button>
          ) : null
        }
      />

      <Title heading={5} style={{ margin: '20px 0 8px' }}>
        {t('参拍封禁')}
      </Title>
      <ListPage<BanRow>
        key={`n-${reloadKey}`}
        api="/promotion/auction/ban/list"
        rowKey="banId"
        pageSize={10}
        actionWidth={110}
        filters={[{ field: 'userId', label: t('会员ID'), type: 'number', width: 140 }]}
        columns={banColumns}
        rowActions={(row) =>
          row.status === 1 ? (
            <Button theme="borderless" size="small" onClick={() => unban(row)}>
              {t('解封')}
            </Button>
          ) : null
        }
        toolbarExtra={
          <Button theme="solid" type="primary" onClick={() => setBanOpen(true)}>
            {t('手动封禁')}
          </Button>
        }
      />

      <Modal
        title={t('扣罚改判豁免')}
        visible={!!waiveRow}
        onCancel={() => setWaiveRow(null)}
        onOk={() => void submitWaive()}
        okText={t('确认改判')}
      >
        <Text type="tertiary">
          {t('改判后，已扣罚的 ¥{amount} 将退回会员余额；参拍封禁不会自动解除，如需解封请在下方封禁列表操作。', {
            amount: String(waiveRow?.penaltyAmount ?? 0),
          })}
        </Text>
        <Input
          style={{ marginTop: 12 }}
          placeholder={t('改判理由（必填，将留痕）')}
          value={waiveReason}
          onChange={setWaiveReason}
        />
      </Modal>

      <Modal
        title={t('手动封禁参拍资格')}
        visible={banOpen}
        onCancel={() => setBanOpen(false)}
        onOk={() => void submitBan()}
        okText={t('确认封禁')}
      >
        <Space vertical align="start" style={{ width: '100%' }} spacing={12}>
          <Space>
            <Text>{t('会员ID')}</Text>
            <InputNumber style={{ width: 160 }} value={banUserId} onChange={(v) => setBanUserId(v == null ? undefined : Number(v))} />
          </Space>
          <Space>
            <Text>{t('封禁类型')}</Text>
            <Select
              style={{ width: 160 }}
              value={banType}
              onChange={(v) => setBanType(Number(v))}
              optionList={[
                { label: t('临时封禁'), value: 1 },
                { label: t('永久封禁'), value: 2 },
              ]}
            />
            {banType === 1 ? (
              <>
                <InputNumber style={{ width: 100 }} min={1} value={banDays} onChange={(v) => setBanDays(Number(v ?? 30))} />
                <Text>{t('天')}</Text>
              </>
            ) : null}
          </Space>
          <Input style={{ width: '100%' }} placeholder={t('封禁原因')} value={banReason} onChange={setBanReason} />
        </Space>
      </Modal>
    </div>
  );
}
