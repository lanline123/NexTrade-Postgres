import { useState } from 'react';
import { Avatar, Button, Modal, Space, Tag, Toast, Typography } from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import type { ColumnConfig } from '../framework/types';
import ImageUploader from '../components/ImageUploader';

const { Text } = Typography;

interface AdminUserRow {
  adminId: number;
  username: string;
  avatar?: string;
  roleId?: number;
  roleName?: string;
  email?: string;
  mobile?: string;
  addTime?: string;
}

export default function AdminUsers() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  // 登录安全（小缺口批 2026-11-21）：后台账号登录锁定状态查看/解锁，
  // 与会员详情「登录安全」区块同口径（状态查询 + 手动解除）。
  const [lockTarget, setLockTarget] = useState<AdminUserRow | null>(null);
  const [lockStatus, setLockStatus] = useState<Record<string, any> | null>(null);
  const [lockLoading, setLockLoading] = useState(false);
  const [unlocking, setUnlocking] = useState(false);

  const fetchLockStatus = async (adminId: number) => {
    setLockLoading(true);
    try {
      const result = await apiGet<Record<string, any>>(
        '/authority/adminUser/loginLockStatus',
        { id: adminId },
      );
      setLockStatus(result);
    } catch {
      setLockStatus(null);
    } finally {
      setLockLoading(false);
    }
  };

  const openLockStatus = (record: AdminUserRow) => {
    setLockTarget(record);
    setLockStatus(null);
    void fetchLockStatus(record.adminId);
  };

  const unlockLogin = async () => {
    if (!lockTarget) return;
    setUnlocking(true);
    try {
      await apiPost('/authority/adminUser/unlockLogin', { id: lockTarget.adminId });
      Toast.success(t('已解除登录锁定'));
      void fetchLockStatus(lockTarget.adminId);
    } catch {
      Toast.error(t('解除失败，请稍后重试'));
    } finally {
      setUnlocking(false);
    }
  };

  const columns: ColumnConfig<AdminUserRow>[] = [
    {
      title: '管理员名称',
      dataIndex: 'username',
      width: 220,
      render: (value: string, record) => (
        <Space>
          <Avatar src={record.avatar} size="small">
            {(value || 'A').slice(0, 1)}
          </Avatar>
          <span>{value}</span>
        </Space>
      ),
    },
    {
      title: '权限组',
      dataIndex: 'roleName',
      width: 180,
      render: (value: string, record) =>
        record.roleId === 0 ? (
          <Tag color="blue">{t('自定义权限')}</Tag>
        ) : (
          <Tag color="blue">{value || '-'}</Tag>
        ),
    },
    { title: '邮箱', dataIndex: 'email', width: 220, render: (value: string) => value || '-' },
    { title: '电话', dataIndex: 'mobile', width: 160, render: (value: string) => value || '-' },
    { title: '添加时间', dataIndex: 'addTime', width: 180, render: (value: string) => value || '-' },
  ];

  const batchDelete = (ids: number[], clear: () => void) => {
    Modal.confirm({
      title: t('批量删除管理员'),
      content: t('确定删除选中的 {count} 个管理员？', { count: ids.length }),
      onOk: async () => {
        try {
          await apiPost('/authority/adminUser/batch', { ids, type: 'del' });
          Toast.success({ content: t('批量删除成功') });
          clear();
          setReloadKey((key) => key + 1);
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('批量删除失败') });
        }
      },
    });
  };

  return (
    <>
    <ListPage<AdminUserRow>
      key={reloadKey}
      api="/authority/adminUser/list"
      rowKey="adminId"
      pageSize={10}
      selectable
      actionWidth={210}
      filters={[
        {
          field: 'keyword',
          label: '管理员',
          type: 'input',
          placeholder: '输入管理员名称',
          width: 260,
        },
      ]}
      columns={columns}
      rowActions={(record) => (
        <Space>
          <Button theme="borderless" size="small" onClick={() => openLockStatus(record)}>
            {t('登录安全')}
          </Button>
          <Button
            theme="borderless"
            size="small"
            onClick={() => {
              const url = new URL(window.location.href);
              url.searchParams.set('page', 'adminLogManage');
              url.searchParams.set('keyword', record.username);
              window.location.href = url.toString();
            }}
          >
            {t('日志')}
          </Button>
        </Space>
      )}
      batchActions={(rows, clear) => (
        <Space>
          <Button size="small" onClick={() => batchDelete(rows.map((row) => row.adminId), clear)}>
            {t('批量删除')}
          </Button>
        </Space>
      )}
      drawer={{
        title: '管理员',
        addText: '添加管理员',
        width: 700,
        labelWidth: 'auto',
        inputWidth: 420,
        createApi: '/authority/adminUser/create',
        updateApi: '/authority/adminUser/update',
        detailApi: '/authority/adminUser/detail',
        deleteApi: '/authority/adminUser/del',
        idField: 'adminId',
        sections: [
          {
            key: 'basic',
            title: '管理员信息',
            fields: [
              {
                field: 'username',
                label: t('管理员名称'),
                required: true,
                placeholder: t('请输入管理员名称'),
              },
              {
                field: 'mobile',
                label: t('手机号'),
                required: true,
                placeholder: t('请输入手机号'),
              },
              { field: 'email', label: t('邮箱'), placeholder: t('请输入邮箱') },
              {
                field: 'avatar',
                label: t('头像'),
                type: 'custom',
                render: ({ formApi, values }) => (
                  <ImageUploader
                    value={values.avatar ? [values.avatar] : []}
                    onChange={(urls) => formApi?.setValue('avatar', urls[0] || '')}
                    max={1}
                  />
                ),
              },
              {
                field: 'password',
                label: t('设置密码'),
                placeholder: t('编辑时留空不修改'),
                visibleWhen: (mode) => mode === 'create',
                requiredWhen: (mode) => mode === 'create',
              },
              {
                field: 'pwdConfirm',
                label: t('确认密码'),
                placeholder: t('请再次输入密码'),
                visibleWhen: (mode) => mode === 'create',
                requiredWhen: (mode) => mode === 'create',
              },
              {
                field: 'roleId',
                label: t('管理员角色'),
                type: 'select',
                placeholder: t('请选择角色'),
                optionsApi: '/authority/adminRole/list',
                optionsParams: { page: 1, pageSize: 100 },
                optionLabel: 'roleName',
                optionValue: 'roleId',
              },
            ],
          },
        ],
        toPayload: (values) => ({
          ...values,
          adminId: values.adminId ? Number(values.adminId) : undefined,
          roleId: values.roleId ? Number(values.roleId) : undefined,
        }),
      }}
    />
    <Modal
      title={t('登录安全')}
      visible={!!lockTarget}
      onCancel={() => setLockTarget(null)}
      footer={null}
      width={420}
    >
      {lockTarget && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <Text strong>{lockTarget.username}</Text>
          </div>
          {lockLoading || !lockStatus ? (
            <Text type="tertiary">{t('加载中…')}</Text>
          ) : lockStatus.locked ? (
            <Space vertical align="start" spacing="medium">
              <Space>
                <Tag color="red">{t('登录已锁定')}</Tag>
                <Text type="tertiary">
                  {`${t('约')} ${Math.max(1, Math.ceil(Number(lockStatus.remainingSeconds || 0) / 60))} ${t('分钟后自动解除')}`}
                </Text>
              </Space>
              <Button theme="solid" type="primary" loading={unlocking} onClick={unlockLogin}>
                {t('手动解除锁定')}
              </Button>
            </Space>
          ) : (
            <Text type="tertiary">
              {lockStatus.enabled
                ? `${t('未锁定')} · ${t('当前窗口内失败')} ${Number(lockStatus.failCount || 0)} / ${Number(lockStatus.maxFailures || 5)} ${t('次')}`
                : t('登录失败锁定未启用')}
            </Text>
          )}
        </div>
      )}
    </Modal>
    </>
  );
}
