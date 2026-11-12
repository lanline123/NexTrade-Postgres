import { useEffect, useState } from 'react';
import {
  Button,
  Input,
  InputNumber,
  Modal,
  Space,
  Spin,
  Table,
  Tabs,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import ListPage from '../framework/ListPage';
import FormDrawer from '../framework/FormDrawer';
import ImageUploader from '../components/ImageUploader';
import MultilingualField from '../components/MultilingualField';
import GrouponItemManager, { type GrouponItemRow } from '../components/GrouponItemManager';
import GrouponItemEditor from '../components/GrouponItemEditor';
import SendUserSelector, { type SendUserTarget } from '../components/SendUserSelector';
import type { ColumnConfig, FormDrawerConfig } from '../framework/types';

const { Text } = Typography;

interface GrouponRow {
  grouponPromotionId: number;
  name?: string;
  startTime?: string;
  endTime?: string;
  groupType?: number;
  receiveAmount?: number | string;
  groupSuccessCount?: number;
  payOrderCount?: number;
  status?: number;
  statusName?: string;
}

const DEFAULT_FORM = {
  name: '',
  startTime: '',
  endTime: '',
  groupType: 1,
  expirationDays: 1,
  expirationHours: 0,
  expirationMinutes: 30,
  pcGrouponImage: '',
  grouponImage: '',
  shareImage: '',
  grouponExplain: '',
  sendUserTarget: { scope: 0, userIds: [], users: [], rankIds: [], tagIds: [], tagMatchMode: 0 } as SendUserTarget,
};

/** 拼团活动：对应旧版 promotion/groupon/List.vue + Info.vue + item/* + record/*。
 * 已实现：活动 CRUD、拼团商品管理（含 SKU 明细/阶梯价/虚拟成团）、
 * 参与记录（进行中的团 + 手动/批量成团 + 订单记录）、推广海报。 */
export default function Groupons() {
  const { t } = useI18n();
  const [reloadKey, setReloadKey] = useState(0);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<'create' | 'edit'>('create');
  const [drawerInitial, setDrawerInitial] = useState<Record<string, any>>({ ...DEFAULT_FORM });
  const [editingId, setEditingId] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [itemTarget, setItemTarget] = useState<GrouponRow | null>(null);
  const [itemRefreshKey, setItemRefreshKey] = useState(0);
  const [recordTarget, setRecordTarget] = useState<GrouponRow | null>(null);
  const [recordRows, setRecordRows] = useState<any[]>([]);
  const [recordLoading, setRecordLoading] = useState(false);
  const [recordTab, setRecordTab] = useState('join');
  const [selectedRecordIds, setSelectedRecordIds] = useState<number[]>([]);
  const [memberRows, setMemberRows] = useState<any[]>([]);
  const [memberLoading, setMemberLoading] = useState(false);
  const [memberTarget, setMemberTarget] = useState<any | null>(null);
  const [itemEditOpen, setItemEditOpen] = useState(false);
  const [itemEditInitial, setItemEditInitial] = useState<Record<string, any> | null>(null);
  const [itemEditSeq, setItemEditSeq] = useState(0);
  const [itemSaving, setItemSaving] = useState(false);
  const [postTarget, setPostTarget] = useState<any | null>(null);
  const [postInfo, setPostInfo] = useState<Record<string, any> | null>(null);
  const [postLoading, setPostLoading] = useState(false);

  const reload = () => setReloadKey((key) => key + 1);

  const openCreate = () => {
    setDrawerMode('create');
    setEditingId(null);
    setDrawerInitial({ ...DEFAULT_FORM });
    setDrawerOpen(true);
  };

  const openEdit = async (row: GrouponRow) => {
    setDrawerMode('edit');
    setEditingId(row.grouponPromotionId);
    try {
      const detail = await apiGet<any>('/promotion/groupon/detail', { id: row.grouponPromotionId });
      const expiration = Number(detail?.expirationTime ?? 1800);
      setDrawerInitial({
        ...DEFAULT_FORM,
        ...detail,
        expirationDays: Math.floor(expiration / 86400),
        expirationHours: Math.floor((expiration % 86400) / 3600),
        expirationMinutes: Math.floor((expiration % 3600) / 60),
      });
    } catch {
      setDrawerInitial({ ...DEFAULT_FORM, ...row });
    }
    setDrawerOpen(true);
  };

  const saveGroupon = async (payload: Record<string, any>) => {
    if (!String(payload.name || '').trim()) {
      Toast.error({ content: t('活动名称不能为空') });
      return;
    }
    const expirationTime =
      Number(payload.expirationDays ?? 0) * 86400 +
      Number(payload.expirationHours ?? 0) * 3600 +
      Number(payload.expirationMinutes ?? 0) * 60;
    if (expirationTime < 900 || expirationTime > 30 * 86400) {
      Toast.error({ content: t('拼团有效期需在15分钟至30天之间') });
      return;
    }
    setSaving(true);
    try {
      const body: Record<string, any> = {
        name: payload.name,
        startTime: payload.startTime,
        endTime: payload.endTime,
        groupType: Number(payload.groupType ?? 1),
        expirationTime,
        pcGrouponImage: payload.pcGrouponImage || '',
        grouponImage: payload.grouponImage || '',
        shareImage: payload.shareImage || '',
        grouponExplain: payload.grouponExplain || '',
        // 参与人群（对齐旧版 SendUserSelector → memberRange/memberMarks/sendUserTarget）
        sendUserTarget:
          (payload.sendUserTarget as SendUserTarget) ||
          { scope: 0, userIds: [], users: [], rankIds: [], tagIds: [], tagMatchMode: 0 },
        memberRange:
          (payload.sendUserTarget as SendUserTarget | undefined)?.scope === 1 ? 1 : 0,
        memberMarks:
          (payload.sendUserTarget as SendUserTarget | undefined)?.scope === 1
            ? (payload.sendUserTarget as SendUserTarget).tagIds || []
            : [],
      };
      if (drawerMode === 'edit' && editingId) {
        await apiPost('/promotion/groupon/update', { ...body, grouponPromotionId: editingId });
      } else {
        await apiPost('/promotion/groupon/create', body);
      }
      Toast.success({ content: drawerMode === 'edit' ? t('更新成功') : t('创建成功') });
      setDrawerOpen(false);
      reload();
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
    } finally {
      setSaving(false);
    }
  };

  const finishGroupon = (row: GrouponRow) => {
    Modal.confirm({
      title: t('结束进行中的活动会影响粉丝正常购买，建议您提前告知或说明，确定要结束活动吗？'),
      onOk: async () => {
        try {
          await apiPost('/promotion/groupon/finish', { id: row.grouponPromotionId });
          Toast.success({ content: t('停止活动成功！') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const deleteGroupon = (row: GrouponRow) => {
    Modal.confirm({
      title: t('确认删除该活动？活动删除后，将不能恢复。'),
      onOk: async () => {
        try {
          await apiPost('/promotion/groupon/del', { id: row.grouponPromotionId });
          Toast.success({ content: t('删除成功') });
          reload();
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const loadRecords = async (row: GrouponRow, tab = recordTab) => {
    setRecordLoading(true);
    setSelectedRecordIds([]);
    try {
      const api =
        tab === 'join'
          ? '/promotion/groupon/record/openJoinList'
          : '/promotion/groupon/record/orderList';
      const result = await apiGet<any>(api, {
        grouponPromotionId: row.grouponPromotionId,
        page: 1,
        size: 50,
      });
      setRecordRows(result?.records || result?.list || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    } finally {
      setRecordLoading(false);
    }
  };

  /** 打开添加/编辑弹窗。
   * 修复点：
   * 1. 详情参数是 `grouponPromotionItemId`（旧代码传 `id`，后端必 500「未知的拼团活动商品」）；
   * 2. 详情拉齐后才打开弹窗（旧实现先开再异步回填，state 只初始化一次，连续新建/编辑会串数据）；
   * 3. 每次打开都重置 initial，编辑器组件以 key 重建，保证状态干净。 */
  const openItemEdit = async (groupon: GrouponRow, item?: GrouponItemRow) => {
    setItemTarget(groupon);
    setItemEditOpen(false);
    setItemEditInitial(null);
    let next: Record<string, any> | null;
    if (item) {
      const itemId = item.grouponPromotionItemId ?? item.itemId;
      try {
        const detail = await apiGet<any>('/promotion/groupon/item/detail', {
          grouponPromotionItemId: itemId,
        });
        next = { ...item, ...detail };
      } catch (error) {
        Toast.error({ content: error instanceof Error ? error.message : t('详情加载失败') });
        next = { ...item };
      }
    } else {
      next = {
        grouponPromotionId: groupon.grouponPromotionId,
        groupType: Number(groupon.groupType ?? 1),
        orderLimitNum: 0,
        isAutoGroup: 0,
        minGroupNum: 2,
        sort: 0,
        groupNums: [{ groupNumKey: 0, groupNum: 2 }],
      };
    }
    setItemEditInitial(next);
    setItemEditSeq((seq) => seq + 1);
    setItemEditOpen(true);
  };

  const showPost = async (item: any) => {
    setPostTarget(item);
    setPostLoading(true);
    try {
      const result = await apiGet<any>('/promotion/groupon/item/post', {
        grouponPromotionItemId: item.itemId ?? item.grouponPromotionItemId,
      });
      setPostInfo(result || {});
    } catch (error) {
      setPostInfo(null);
      Toast.error({ content: error instanceof Error ? error.message : t('海报加载失败') });
    } finally {
      setPostLoading(false);
    }
  };

  const manualGroup = (record: any) => {
    Modal.confirm({
      title: t('确认手动成团？成团后不可恢复。'),
      onOk: async () => {
        try {
          await apiPost(
            `/promotion/groupon/record/manualGroup?grouponPromotionRecordId=${record.grouponPromotionRecordId}`,
          );
          Toast.success({ content: t('成团成功') });
          if (recordTarget) loadRecords(recordTarget);
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const batchGroup = () => {
    if (selectedRecordIds.length === 0) return;
    Modal.confirm({
      title: t('确认对所选 {count} 个团执行手动成团？', { count: selectedRecordIds.length }),
      onOk: async () => {
        try {
          await apiPost('/promotion/groupon/record/batchGroup', selectedRecordIds);
          Toast.success({ content: t('批量成团成功') });
          if (recordTarget) loadRecords(recordTarget);
        } catch (error) {
          Toast.error({ content: error instanceof Error ? error.message : t('操作失败') });
        }
      },
    });
  };

  const showMembers = async (record: any) => {
    setMemberTarget(record);
    setMemberLoading(true);
    try {
      const result = await apiGet<any>('/promotion/groupon/record/groupMemberList', {
        grouponPromotionRecordId: record.grouponPromotionRecordId,
      });
      setMemberRows(Array.isArray(result) ? result : result?.records || []);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载团员失败') });
      setMemberRows([]);
    } finally {
      setMemberLoading(false);
    }
  };

  const columns: ColumnConfig<GrouponRow>[] = [
    {
      title: '活动名称',
      dataIndex: 'name',
      width: 170,
      render: (value: string) => <span>{value || '-'}</span>,
    },
    {
      title: '活动时间',
      dataIndex: 'startTime',
      width: 320,
      render: (_: unknown, row: GrouponRow) => (
        <span>
          {row.startTime || '-'}{t('至')}{row.endTime || '-'}
        </span>
      ),
    },
    {
      title: '拼团类型',
      dataIndex: 'groupType',
      width: 100,
      render: (value: number) =>
        Number(value) === 1 ? <Tag color="blue">{t('普通团')}</Tag> : Number(value) === 2 ? <Tag color="purple">{t('阶梯团')}</Tag> : <span>-</span>,
    },
    { title: '应收金额', dataIndex: 'receiveAmount', width: 110 },
    { title: '成团数', dataIndex: 'groupSuccessCount', width: 90 },
    { title: '支付单数', dataIndex: 'payOrderCount', width: 90 },
    {
      title: '活动状态',
      dataIndex: 'statusName',
      width: 110,
      render: (value: string) => <span>{value || '-'}</span>,
    },
  ];

  const drawerConfig: FormDrawerConfig = {
    createApi: '/promotion/groupon/create',
    title: '拼团活动',
    width: 720,
    labelWidth: 120,
    sections: [
      {
        key: 'basic',
        title: '基础信息',
        fields: [
          {
            field: 'name',
            label: t('活动名称'),
            required: true,
            placeholder: t('请输入活动名称'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <MultilingualField
                value={values.name}
                onChange={(next: string) => formApi?.setValue('name', next)}
                dataId={editingId ?? 0}
                dataType={4}
                maxlength={60}
                placeholder={t('请输入活动名称')}
              />
            ),
          },
          {
            field: 'activityTime',
            label: t('活动时间'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <Space>
                <Input
                  style={{ width: 200 }}
                  placeholder={t('开始时间 YYYY-MM-DD HH:mm:ss')}
                  value={values.startTime || ''}
                  onChange={(next) => formApi?.setValue('startTime', next)}
                />
                <Input
                  style={{ width: 200 }}
                  placeholder={t('结束时间 YYYY-MM-DD HH:mm:ss')}
                  value={values.endTime || ''}
                  onChange={(next) => formApi?.setValue('endTime', next)}
                />
              </Space>
            ),
          },
          {
            field: 'groupType',
            label: t('拼团类型'),
            type: 'radio',
            options: [
              { label: t('普通团'), value: 1 },
              { label: t('阶梯团'), value: 2 },
            ],
          },
          {
            field: 'expirationTime',
            label: t('成团有效时间'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <div>
                <Space>
                  <InputNumber
                    style={{ width: 110 }}
                    min={0}
                    precision={0}
                    value={Number(values.expirationDays ?? 0)}
                    onChange={(next) => formApi?.setValue('expirationDays', Number(next ?? 0))}
                    addonAfter={t('日')}
                  />
                  <InputNumber
                    style={{ width: 110 }}
                    min={0}
                    precision={0}
                    value={Number(values.expirationHours ?? 0)}
                    onChange={(next) => formApi?.setValue('expirationHours', Number(next ?? 0))}
                    addonAfter={t('时')}
                  />
                  <InputNumber
                    style={{ width: 110 }}
                    min={0}
                    precision={0}
                    value={Number(values.expirationMinutes ?? 0)}
                    onChange={(next) => formApi?.setValue('expirationMinutes', Number(next ?? 0))}
                    addonAfter={t('分')}
                  />
                </Space>
                <p className="groupon-form-extra">
                  {t('拼团有效期需在15分钟至30天之间，若设置1日，用户开团后，需要在1日内成团，超时则拼团失败')}
                </p>
              </div>
            ),
          },
        ],
      },
      {
        key: 'audience',
        title: '参与人群',
        fields: [
          {
            field: 'sendUserTarget',
            label: t('参与人群'),
            type: 'custom',
            span: 24,
            render: ({ formApi, values }) => (
              <SendUserSelector
                value={values.sendUserTarget as SendUserTarget}
                onChange={(next) => formApi?.setValue('sendUserTarget', next)}
              />
            ),
          },
        ],
      },
      {
        key: 'images',
        title: '活动图片与规则',
        fields: [
          {
            field: 'pcGrouponImage',
            label: t('PC活动图片'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <ImageUploader
                single
                value={values.pcGrouponImage ? [values.pcGrouponImage] : []}
                onChange={(urls) => formApi?.setValue('pcGrouponImage', urls[0] || '')}
              />
            ),
          },
          {
            field: 'grouponImage',
            label: t('H5活动图片'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <ImageUploader
                single
                value={values.grouponImage ? [values.grouponImage] : []}
                onChange={(urls) => formApi?.setValue('grouponImage', urls[0] || '')}
              />
            ),
          },
          {
            field: 'shareImage',
            label: t('专题页分享图'),
            type: 'custom',
            render: ({ formApi, values }) => (
              <ImageUploader
                single
                value={values.shareImage ? [values.shareImage] : []}
                onChange={(urls) => formApi?.setValue('shareImage', urls[0] || '')}
              />
            ),
          },
          { field: 'grouponExplain', label: t('活动规则'), type: 'textarea' },
        ],
      },
    ],
    toPayload: (values) => {
      if (!String(values.name || '').trim()) throw new Error(t('活动名称不能为空'));
      return values;
    },
  };

  return (
    <div className="groupons-page">
      <ListPage<GrouponRow>
        key={reloadKey}
        api="/promotion/groupon/list"
        rowKey="grouponPromotionId"
        pageSize={10}
        actionWidth={300}
        filters={[
          { field: 'grouponName', label: '活动名称', type: 'input', placeholder: '输入活动名称', width: 240 },
        ]}
        columns={columns}
        rowActions={(row) => (
          <Space spacing={2} wrap>
            <Button theme="borderless" size="small" onClick={() => openEdit(row)}>
              {t('编辑活动')}
            </Button>
            <Button
              theme="borderless"
              size="small"
              onClick={() => {
                setItemTarget(row);
              }}
            >
              {t('管理商品')}
            </Button>
            <Button
              theme="borderless"
              size="small"
              onClick={() => {
                setRecordTarget(row);
                setRecordTab('join');
                loadRecords(row, 'join');
              }}
            >
              {t('参与记录')}
            </Button>
            {Number(row.status) !== 2 ? (
              <Button theme="borderless" type="danger" size="small" onClick={() => finishGroupon(row)}>
                {t('立即结束')}
              </Button>
            ) : (
              <Button theme="borderless" type="danger" size="small" onClick={() => deleteGroupon(row)}>
                {t('删除')}
              </Button>
            )}
          </Space>
        )}
        toolbarExtra={
          <Button theme="solid" type="primary" onClick={openCreate}>
            {t('添加拼团活动')}
          </Button>
        }
      />

      <FormDrawer
        visible={drawerOpen}
        mode={drawerMode}
        initialValues={drawerInitial}
        config={drawerConfig}
        saving={saving}
        onCancel={() => setDrawerOpen(false)}
        onSubmit={saveGroupon}
      />

      {/* 拼团商品管理（现代化弹窗，见 components/GrouponItemManager.tsx） */}
      <GrouponItemManager
        visible={Boolean(itemTarget) && !itemEditOpen}
        groupon={itemTarget}
        refreshToken={itemRefreshKey}
        onClose={() => setItemTarget(null)}
        onEdit={(item) => itemTarget && openItemEdit(itemTarget, item)}
        onPoster={(item) => showPost(item)}
      />

      {/* 参与记录 */}
      <Modal
        title={recordTarget ? `${t('参与记录')}：${recordTarget.name || ''}` : t('参与记录')}
        visible={Boolean(recordTarget)}
        footer={null}
        onCancel={() => setRecordTarget(null)}
        width={860}
      >
        <Tabs
          type="card"
          activeKey={recordTab}
          onChange={(key: any) => {
            setRecordTab(String(key));
            if (recordTarget) loadRecords(recordTarget, key as string);
          }}
          tabList={[
            { itemKey: 'join', tab: t('进行中的团') },
            { itemKey: 'order', tab: t('订单记录') },
          ]}
          style={{ marginBottom: 8 }}
        />
        {recordTab === 'join' && (
          <Space style={{ marginBottom: 8 }}>
            <Button disabled={!selectedRecordIds.length} onClick={batchGroup}>
              {t('批量成团')}
            </Button>
            <Text type="tertiary" size="small">
              {t('对未满员且已到期的团可手动成团')}
            </Text>
          </Space>
        )}
        <Spin spinning={recordLoading}>
          {recordTab === 'join' ? (
            <Table
              size="small"
              pagination={false}
              rowKey={(record: any, index?: number) => String(record.grouponPromotionRecordId ?? index)}
              dataSource={recordRows}
              rowSelection={{
                selectedRowKeys: selectedRecordIds,
                onChange: (keys) => setSelectedRecordIds((keys || []).map(Number)),
              }}
              columns={[
                {
                  title: t('开团时间'),
                  dataIndex: 'openGroupTime',
                  width: 170,
                  render: (v: any) => (v ? String(v).replace('T', ' ').slice(0, 19) : '-'),
                },
                { title: t('商品名称'), dataIndex: 'productName' },
                { title: t('团长'), dataIndex: 'groupCaptainName', width: 120, render: (v: any) => v || '-' },
                {
                  title: t('团人数'),
                  dataIndex: 'minGroupNum',
                  width: 90,
                },
                {
                  title: t('状态'),
                  dataIndex: 'joinStatus',
                  width: 100,
                  render: (v: number) =>
                    Number(v) === 1 ? <Tag color="green">{t('已成团')}</Tag> : <Tag color="orange">{t('拼团中')}</Tag>,
                },
                {
                  title: t('操作'),
                  width: 160,
                  render: (_v: any, r: any) => (
                    <Space spacing={2}>
                      <Button theme="borderless" size="small" onClick={() => showMembers(r)}>
                        {t('查看团员')}
                      </Button>
                      <Button theme="borderless" size="small" onClick={() => manualGroup(r)}>
                        {t('手动成团')}
                      </Button>
                    </Space>
                  ),
                },
              ]}
            />
          ) : (
            <Table
              size="small"
              pagination={false}
              rowKey={(record: any, index?: number) => String(record.orderId ?? record.orderSn ?? index)}
              dataSource={recordRows}
              columns={[
                { title: t('订单号'), dataIndex: 'orderSn', render: (v: any) => <span className="mono">{v || '-'}</span> },
                { title: t('参团会员'), dataIndex: 'groupCaptainName', render: (v: any) => v || '-' },
                { title: t('参团方式'), dataIndex: 'joinTypeText', render: (v: any) => v || '-' },
                { title: t('状态'), dataIndex: 'statusText', render: (v: any) => v || '-' },
              ]}
            />
          )}
          {!recordRows.length && !recordLoading ? <Text type="tertiary">{t('暂无参与记录')}</Text> : null}
        </Spin>
      </Modal>

      {/* 团员列表 */}
      <Modal
        title={t('团成员')}
        visible={Boolean(memberTarget)}
        footer={null}
        onCancel={() => setMemberTarget(null)}
        width={560}
      >
        <Spin spinning={memberLoading}>
          <Table
            size="small"
            pagination={false}
            rowKey={(record: any, index?: number) => String(record.id ?? index)}
            dataSource={memberRows}
            columns={[
              { title: t('会员'), dataIndex: 'nickname', render: (v: any, r: any) => v || r.username || '-' },
              { title: t('是否团长'), dataIndex: 'isCaptain', width: 100, render: (v: any) => (Number(v) === 1 ? t('是') : t('否')) },
              { title: t('参团时间'), dataIndex: 'addTime', width: 170, render: (v: any) => v || '-' },
            ]}
          />
        </Spin>
      </Modal>

      {/* 推广海报 */}
      <Modal
        title={t('推广海报')}
        visible={Boolean(postTarget)}
        footer={null}
        onCancel={() => {
          setPostTarget(null);
          setPostInfo(null);
        }}
        width={520}
      >
        <Spin spinning={postLoading}>
          {postInfo ? (
            <div style={{ display: 'grid', gap: 10 }}>
              {postInfo.promotionPicUrl ? (
                <img src={postInfo.promotionPicUrl} alt="" style={{ maxWidth: '100%', borderRadius: 6 }} />
              ) : null}
              <div>
                <Text type="tertiary">{t('小程序码')}</Text>
                {postInfo.miniCode ? (
                  <img src={postInfo.miniCode} alt="" style={{ width: 140, display: 'block' }} />
                ) : (
                  <span> --</span>
                )}
              </div>
              {postInfo.shortLink ? (
                <div>
                  <Text type="tertiary">{t('短链接')}</Text>
                  <div className="mono">{postInfo.shortLink}</div>
                </div>
              ) : null}
              {postInfo.promotionBrief ? (
                <div>
                  <Text type="tertiary">{t('活动简介')}</Text>
                  <div>{postInfo.promotionBrief}</div>
                </div>
              ) : null}
            </div>
          ) : null}
        </Spin>
      </Modal>

      {/* 拼团商品添加/编辑（现代化弹窗，见 components/GrouponItemEditor.tsx）。
          key=itemEditSeq：每次打开强制重建，杜绝旧 state 串数据。 */}
      {itemTarget && itemEditInitial && (
        <GrouponItemEditor
          key={itemEditSeq}
          visible={itemEditOpen}
          groupon={itemTarget}
          initial={itemEditInitial}
          saving={itemSaving}
          onCancel={() => {
            setItemEditOpen(false);
            setItemEditInitial(null);
          }}
          onSaved={() => {
            setItemEditOpen(false);
            setItemEditInitial(null);
            setItemSaving(false);
            setItemRefreshKey((key) => key + 1);
          }}
          onSavingChange={setItemSaving}
        />
      )}
    </div>
  );
}
