import { useCallback, useEffect, useState } from 'react';
import { Button, Input, Modal, Space, Table, Toast } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import FormDrawer from '../framework/FormDrawer';
import type { FormDrawerConfig } from '../framework/types';

interface SessionRow {
  sessionId: number;
  sessionName: string;
  coverPic?: string;
  startTime?: number;
  endTime?: number;
  startTimeText?: string;
  endTimeText?: string;
  itemCount?: number;
  sort?: number;
  status?: number;
  statusName?: string;
}

const stampToText = (stamp?: number) => {
  if (!stamp) return '';
  const d = new Date(stamp * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
};

const textToStamp = (value: string) => Math.floor(new Date(value.replace(/-/g, '/')).getTime() / 1000);

/** 拍卖场次管理（商品 → 拍卖 → 拍卖场次）。场次仅作组织/展示分组，接口返回全量数组。 */
export default function AuctionSessions() {
  const { t } = useI18n();
  const [rows, setRows] = useState<SessionRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<'create' | 'edit'>('create');
  const [drawerInitial, setDrawerInitial] = useState<Record<string, any>>({});
  const [editingId, setEditingId] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await apiGet<SessionRow[]>('/promotion/auction/session/list');
      setRows(Array.isArray(data) ? data : []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const openCreate = () => {
    setDrawerMode('create');
    setEditingId(null);
    setDrawerInitial({ sessionName: '', coverPic: '', startText: '', endText: '', sort: 0, status: true });
    setDrawerOpen(true);
  };

  const openEdit = (row: SessionRow) => {
    setDrawerMode('edit');
    setEditingId(row.sessionId);
    setDrawerInitial({
      sessionName: row.sessionName || '',
      coverPic: row.coverPic || '',
      startText: stampToText(row.startTime),
      endText: stampToText(row.endTime),
      sort: row.sort ?? 0,
      status: (row.status ?? 1) === 1,
    });
    setDrawerOpen(true);
  };

  const save = async (payload: Record<string, any>) => {
    if (!String(payload.sessionName || '').trim()) {
      Toast.error({ content: t('场次名称不能为空') });
      return;
    }
    setSaving(true);
    try {
      await apiPost('/promotion/auction/session/save', {
        sessionId: drawerMode === 'edit' ? editingId : undefined,
        sessionName: payload.sessionName,
        coverPic: payload.coverPic || '',
        startTime: payload.startText ? textToStamp(String(payload.startText)) : undefined,
        endTime: payload.endText ? textToStamp(String(payload.endText)) : undefined,
        sort: Number(payload.sort ?? 0),
        status: payload.status ? 1 : 0,
      });
      Toast.success({ content: t('保存成功') });
      setDrawerOpen(false);
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const remove = (row: SessionRow) => {
    Modal.confirm({
      title: t('确认删除该场次吗？场次下已有拍品时不可删除。'),
      onOk: async () => {
        try {
          await apiPost('/promotion/auction/session/del', { sessionId: row.sessionId });
          Toast.success({ content: t('删除成功') });
          void load();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
        }
      },
    });
  };

  const columns = [
    { title: t('场次名称'), dataIndex: 'sessionName', width: 220 },
    { title: t('拍品数'), dataIndex: 'itemCount', width: 90 },
    { title: t('开始时间'), dataIndex: 'startTimeText', width: 170 },
    { title: t('结束时间'), dataIndex: 'endTimeText', width: 170 },
    { title: t('排序'), dataIndex: 'sort', width: 80 },
    {
      title: t('状态'),
      dataIndex: 'statusName',
      width: 100,
      render: (value: string) => <span>{value ? t(value) : '-'}</span>,
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 140,
      render: (_: unknown, row: SessionRow) => (
        <Space spacing={2}>
          <Button theme="borderless" size="small" onClick={() => openEdit(row)}>
            {t('编辑')}
          </Button>
          <Button theme="borderless" type="danger" size="small" onClick={() => remove(row)}>
            {t('删除')}
          </Button>
        </Space>
      ),
    },
  ];

  const drawerConfig: FormDrawerConfig = {
    createApi: '/promotion/auction/session/save',
    title: t('拍卖场次'),
    width: 560,
    labelWidth: 100,
    sections: [
      {
        key: 'basic',
        title: t('场次信息'),
        fields: [
          { field: 'sessionName', label: t('场次名称'), type: 'input', required: true },
          { field: 'coverPic', label: t('封面图地址'), type: 'input', placeholder: t('图片 URL') },
          {
            field: 'timeRange',
            label: t('场次时间'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <Space>
                <Input
                  style={{ width: 190 }}
                  placeholder="YYYY-MM-DD HH:mm:ss"
                  value={values.startText || ''}
                  onChange={(v) => formApi?.setValue('startText', v)}
                />
                <Input
                  style={{ width: 190 }}
                  placeholder="YYYY-MM-DD HH:mm:ss"
                  value={values.endText || ''}
                  onChange={(v) => formApi?.setValue('endText', v)}
                />
              </Space>
            ),
          },
          { field: 'sort', label: t('排序'), type: 'number' },
          { field: 'status', label: t('启用'), type: 'switch', checkedText: t('启用'), uncheckedText: t('停用') },
        ],
      },
    ],
  };

  return (
    <div style={{ padding: 16 }}>
      <div style={{ marginBottom: 12 }}>
        <Button theme="solid" type="primary" onClick={openCreate}>
          {t('新建场次')}
        </Button>
      </div>
      <Table columns={columns} dataSource={rows} loading={loading} rowKey="sessionId" pagination={false} />
      <FormDrawer
        visible={drawerOpen}
        mode={drawerMode}
        initialValues={drawerInitial}
        config={drawerConfig}
        saving={saving}
        onCancel={() => setDrawerOpen(false)}
        onSubmit={save}
      />
    </div>
  );
}
