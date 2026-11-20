// 我方新增（平台联邦 P1）：配对审批页（组织 → 组织管理 → 配对审批）。
import { useCallback, useEffect, useState } from 'react';
import { Button, Card, Input, Modal, Popconfirm, Space, Table, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import { unixToDateTime } from '../utils/format';

const { Text, Title } = Typography;

interface PairingRow {
  id: number;
  expectName?: string;
  status?: number;
  failCount?: number;
  expireTime?: number;
  usedLinkId?: number;
  addTime?: number;
}

interface PairingCreated {
  id: number;
  code: string;
  expectName?: string;
  expireTime?: number;
}

const PAIRING_STATUS: Record<number, { label: string; color: string }> = {
  0: { label: '未用', color: 'blue' },
  1: { label: '已用', color: 'green' },
  2: { label: '作废', color: 'grey' },
};

/** 配对码：生成（明文仅展示一次）+ 近况表 + 作废。 */
export default function HubPairing() {
  const { t } = useI18n();
  const [rows, setRows] = useState<PairingRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [expectName, setExpectName] = useState('');
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<PairingCreated | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await apiGet<PairingRow[]>('/hub/pairing/list');
      setRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载配对码失败') });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const create = async () => {
    if (!expectName.trim()) {
      Toast.warning(t('请填写预期对端名称'));
      return;
    }
    setCreating(true);
    try {
      const result = await apiPost<PairingCreated>('/hub/pairing/create', { expectName: expectName.trim() });
      setCreated(result);
      setExpectName('');
      await load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('生成配对码失败') });
    } finally {
      setCreating(false);
    }
  };

  const revoke = async (record: PairingRow) => {
    try {
      await apiPost('/hub/pairing/revoke', { id: record.id });
      Toast.success(t('已作废'));
      await load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('作废失败') });
    }
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      Toast.success(t('已复制'));
    } catch {
      Toast.warning(t('复制失败，请手动复制'));
    }
  };

  const columns = [
    { title: t('ID'), dataIndex: 'id', width: 80 },
    { title: t('预期名称'), dataIndex: 'expectName', width: 200, render: (v: string) => v || '-' },
    {
      title: t('状态'),
      dataIndex: 'status',
      width: 100,
      render: (v: number) => {
        const item = PAIRING_STATUS[v ?? 0] || PAIRING_STATUS[0];
        return <Tag color={item.color as any}>{t(item.label)}</Tag>;
      },
    },
    { title: t('失败次数'), dataIndex: 'failCount', width: 100, render: (v: number) => v || 0 },
    { title: t('关联连接'), dataIndex: 'usedLinkId', width: 110, render: (v: number) => v || '-' },
    { title: t('过期时间'), dataIndex: 'expireTime', width: 180, render: (v: number) => unixToDateTime(v) },
    { title: t('创建时间'), dataIndex: 'addTime', width: 180, render: (v: number) => unixToDateTime(v) },
    {
      title: t('操作'),
      width: 110,
      render: (_: any, record: PairingRow) =>
        record.status === 0 ? (
          <Popconfirm title={t('确定作废该配对码？')} onConfirm={() => void revoke(record)}>
            <Button theme="borderless" type="danger" size="small">
              {t('作废')}
            </Button>
          </Popconfirm>
        ) : (
          '-'
        ),
    },
  ];

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Title heading={4} style={{ margin: 0 }}>
        {t('配对审批')}
      </Title>
      <Card title={t('生成配对码')}>
        <Space>
          <Input
            value={expectName}
            onChange={setExpectName}
            placeholder={t('预期对端实例名称')}
            style={{ width: 320 }}
          />
          <Button type="primary" theme="solid" loading={creating} onClick={() => void create()}>
            {t('生成配对码')}
          </Button>
        </Space>
        <div style={{ marginTop: 8 }}>
          <Text type="tertiary">{t('配对码明文仅在生成时展示一次，请生成后立即复制并安全交付给对端。')}</Text>
        </div>
      </Card>

      <Card title={t('配对码近况')}>
        <Table loading={loading} dataSource={rows} rowKey="id" columns={columns as any} pagination={false} />
      </Card>

      <Modal
        title={t('配对码已生成（仅展示一次）')}
        visible={Boolean(created)}
        onCancel={() => setCreated(null)}
        footer={
          <Space>
            <Button onClick={() => setCreated(null)}>{t('关闭')}</Button>
            <Button type="primary" theme="solid" onClick={() => created && void copyText(created.code)}>
              {t('复制配对码')}
            </Button>
          </Space>
        }
        width={560}
      >
        <div style={{ display: 'grid', gap: 10 }}>
          <Text type="warning">{t('该配对码明文仅此一次可见，关闭后无法再次查看。')}</Text>
          <div
            style={{
              fontFamily: 'monospace',
              fontSize: 16,
              wordBreak: 'break-all',
              background: 'var(--semi-color-fill-0)',
              padding: 12,
              borderRadius: 6,
            }}
          >
            {created?.code}
          </div>
          <Text type="tertiary">
            {t('预期名称')}：{created?.expectName || '-'} · {t('过期时间')}：{unixToDateTime(created?.expireTime)}
          </Text>
        </div>
      </Modal>
    </div>
  );
}
