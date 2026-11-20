import { useCallback, useEffect, useState } from 'react';
import {
  Button,
  Form,
  Modal,
  Popconfirm,
  Space,
  Spin,
  Switch,
  Table,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';

const { Text, Title } = Typography;

/**
 * 店铺标签（本平台新增，论坛帖 207878955346177229）。
 * 平台标签库：名称 / 颜色 / 排序 / 启停。给店铺打标在「店铺列表 → 标签」完成；
 * 买家侧在店铺名旁展示。与门店服务提示（标签管理 shopTipManage）是两套概念。
 */

interface ShopLabelItem {
  labelId: number;
  labelName: string;
  color: string;
  sortOrder: number;
  status: number;
  addTime: number;
}

const COLOR_PRESETS = ['#E93B3D', '#FF8800', '#F7B500', '#00B578', '#1677FF', '#722ED1', '#595959'];
const DEFAULT_COLOR = '#E93B3D';

export function LabelChip({ name, color }: { name: string; color?: string }) {
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '0 6px',
        height: 18,
        lineHeight: '18px',
        fontSize: 11,
        color: '#fff',
        background: color || DEFAULT_COLOR,
        borderRadius: 0,
        whiteSpace: 'nowrap',
      }}
    >
      {name}
    </span>
  );
}

export default function ShopLabelManage() {
  const { t } = useI18n();
  const [rows, setRows] = useState<ShopLabelItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [editing, setEditing] = useState<ShopLabelItem | null>(null);
  const [color, setColor] = useState(DEFAULT_COLOR);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await apiGet<ShopLabelItem[]>('/merchant/shopLabel/list');
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
    setEditing(null);
    setColor(DEFAULT_COLOR);
    setModalVisible(true);
  };

  const openEdit = (row: ShopLabelItem) => {
    setEditing(row);
    setColor(row.color || DEFAULT_COLOR);
    setModalVisible(true);
  };

  const submit = async (values: Record<string, unknown>) => {
    setSaving(true);
    try {
      const payload = {
        labelId: editing?.labelId,
        labelName: String(values.labelName || '').trim(),
        color,
        sortOrder: Number(values.sortOrder || 0),
        status: values.status === false ? 0 : 1,
      };
      if (editing) {
        await apiPost('/merchant/shopLabel/update', payload);
      } else {
        await apiPost('/merchant/shopLabel/create', payload);
      }
      Toast.success({ content: t('保存成功') });
      setModalVisible(false);
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const toggleStatus = async (row: ShopLabelItem, checked: boolean) => {
    try {
      await apiPost('/merchant/shopLabel/update', {
        labelId: row.labelId,
        labelName: row.labelName,
        color: row.color,
        sortOrder: row.sortOrder,
        status: checked ? 1 : 0,
      });
      setRows((prev) =>
        prev.map((r) => (r.labelId === row.labelId ? { ...r, status: checked ? 1 : 0 } : r)),
      );
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
    }
  };

  const remove = async (row: ShopLabelItem) => {
    try {
      await apiPost('/merchant/shopLabel/del', { labelId: row.labelId });
      Toast.success({ content: t('删除成功') });
      void load();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('删除失败') });
    }
  };

  const columns = [
    {
      title: t('标签'),
      dataIndex: 'labelName',
      width: 220,
      render: (_: unknown, row: ShopLabelItem) => <LabelChip name={row.labelName} color={row.color} />,
    },
    { title: t('颜色'), dataIndex: 'color', width: 140, render: (v: string) => v || '--' },
    { title: t('排序'), dataIndex: 'sortOrder', width: 100 },
    {
      title: t('启用'),
      dataIndex: 'status',
      width: 100,
      render: (v: number, row: ShopLabelItem) => (
        <Switch checked={v === 1} onChange={(checked) => void toggleStatus(row, checked)} />
      ),
    },
    {
      title: t('操作'),
      dataIndex: 'actions',
      width: 160,
      render: (_: unknown, row: ShopLabelItem) => (
        <Space>
          <Button theme="borderless" size="small" onClick={() => openEdit(row)}>
            {t('编辑')}
          </Button>
          <Popconfirm
            title={t('删除该标签？')}
            content={t('删除后所有店铺上的该标签将一并移除。')}
            onConfirm={() => void remove(row)}
          >
            <Button theme="borderless" size="small" type="danger">
              {t('删除')}
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 16 }}>
      <Space style={{ marginBottom: 12, width: '100%', justifyContent: 'space-between' }}>
        <div>
          <Title heading={5} style={{ margin: 0 }}>
            {t('店铺标签')}
          </Title>
          <Text type="tertiary" size="small">
            {t('平台标签库，分配在「店铺列表 → 标签」；标签会展示在买家侧店铺名旁。')}
          </Text>
        </div>
        <Button type="primary" theme="solid" onClick={openCreate}>
          {t('新增标签')}
        </Button>
      </Space>
      <Spin spinning={loading}>
        <Table columns={columns} dataSource={rows} rowKey="labelId" pagination={false} />
      </Spin>

      <Modal
        title={editing ? t('编辑标签') : t('新增标签')}
        visible={modalVisible}
        onCancel={() => setModalVisible(false)}
        footer={null}
        width={440}
      >
        <Form
          key={editing ? `edit-${editing.labelId}` : 'create'}
          initValues={
            editing
              ? { labelName: editing.labelName, sortOrder: editing.sortOrder, status: editing.status === 1 }
              : { labelName: '', sortOrder: 0, status: true }
          }
          onSubmit={(values) => void submit(values as Record<string, unknown>)}
          labelPosition="left"
          labelWidth={72}
        >
          <Form.Input
            field="labelName"
            label={t('名称')}
            maxLength={10}
            showClear
            rules={[{ required: true, message: t('请输入标签名称') }]}
            placeholder={t('如：金牌商家')}
          />
          <Form.Slot label={t('颜色')}>
            <Space wrap>
              {COLOR_PRESETS.map((c) => (
                <span
                  key={c}
                  onClick={() => setColor(c)}
                  style={{
                    width: 22,
                    height: 22,
                    background: c,
                    cursor: 'pointer',
                    border: color === c ? '2px solid #1c1f23' : '2px solid transparent',
                    boxSizing: 'border-box',
                  }}
                />
              ))}
              <span style={{ fontSize: 12, color: 'var(--semi-color-text-2)' }}>{color}</span>
            </Space>
          </Form.Slot>
          <Form.InputNumber field="sortOrder" label={t('排序')} min={0} max={9999} />
          <Form.Switch field="status" label={t('启用')} />
          <div style={{ textAlign: 'right', marginTop: 8 }}>
            <Space>
              <Button onClick={() => setModalVisible(false)}>{t('取消')}</Button>
              <Button type="primary" theme="solid" htmlType="submit" loading={saving}>
                {t('保存')}
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>
    </div>
  );
}
