import { useEffect, useState } from 'react';
import { Button, Card, Image, Input, Table, Toast, Typography } from '@douyinfe/semi-ui';
import { IconPlus } from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { useI18n } from '../i18n';
import FormDrawer from '../framework/FormDrawer';
import type { FormDrawerConfig } from '../framework/types';
import ImageUploader from '../components/ImageUploader';

const { Text } = Typography;

interface RankRow {
  id?: number;
  rankLevel?: number;
  rankName?: string;
  rankLogo?: string;
}

const LEVEL_NAMES = ['Vip1', 'Vip2', 'Vip3', 'Vip4', 'Vip5', 'Vip6', 'Vip7', 'Vip8', 'Vip9', 'Vip10'];

/** 会员等级：对应旧版 user/levelManage/List.vue + Info.vue（批量编辑等级表）。 */
export default function MemberLevels() {
  const { t } = useI18n();
  const [rows, setRows] = useState<RankRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [reloadKey, setReloadKey] = useState(0);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerMode, setDrawerMode] = useState<'create' | 'edit'>('create');
  const [drawerInitial, setDrawerInitial] = useState<Record<string, any>>({});
  const [saving, setSaving] = useState(false);

  const loadList = () => {
    setLoading(true);
    apiGet<any>('/user/userRank/list', { page: 1, size: 50 })
      .then((result) => {
        setRows(result?.userRank?.records || result?.records || []);
      })
      .catch((error) => Toast.error({ content: error.message }))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadList();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reloadKey]);

  const reload = () => setReloadKey((key) => key + 1);

  /** 保存所需的等级配置（rankType/userRankConfig/growUpSetting）：打开编辑时从
   * /user/userRank/detail 回填现值，提交时原样带回（口径同 MemberLevelPros）。 */
  const [rankConfig, setRankConfig] = useState<{
    rankType: number;
    userRankConfig: Record<string, any>;
    growUpSetting: Record<string, any>;
  } | null>(null);

  const openEdit = async () => {
    try {
      // detail 是权威来源：含 rankType 与 growUpSetting（update 校验必填）
      let data: RankRow[] = [];
      try {
        const detail = await apiGet<any>('/user/userRank/detail');
        setRankConfig({
          rankType: Number(detail?.rankType) || 1,
          userRankConfig: {
            type: 1,
            rankAfterMonth: 12,
            useMonth: 12,
            ...(detail?.userRankConfig?.data || {}),
          },
          growUpSetting: detail?.growUpSetting || {
            buyOrder: 0,
            buyOrderNumber: 0,
            buyOrderGrowth: 0,
            evpi: 0,
            evpiGrowth: 0,
            bindPhone: 0,
            bindPhoneGrowth: 0,
          },
        });
        data = (detail?.userRankList || []) as RankRow[];
      } catch {
        setRankConfig(null);
      }
      if (!data.length) {
        const result = await apiGet<any>('/user/userRank/list', { page: 1, size: 50 });
        data = result?.userRank?.records || result?.records || [];
      }
      // 保存统一走 update（见 onSubmit 注释），不再分 create 分支
      setDrawerMode('edit');
      setDrawerInitial({ data: data.map((row) => ({ ...row })) });
      setDrawerOpen(true);
    } catch (error) {
      Toast.error({ content: error instanceof Error ? error.message : t('加载失败') });
    }
  };

  const drawerConfig: FormDrawerConfig = {
    createApi: '/user/userRank/create',
    title: '会员等级',
    width: 640,
    labelWidth: 90,
    sections: [
      {
        key: 'levels',
        title: '等级设置（最多 10 级）',
        fields: [
          {
            field: 'data',
            label: t('等级列表'),
            type: 'custom',
            render: ({ formApi, values }) => {
              const data: RankRow[] = values.data || [];
              return (
                <div className="member-level-rows">
                  {data.map((row, index) => (
                    <div className="member-level-row" key={index}>
                      <span className="member-level-badge">vip{row.rankLevel ?? index + 1}</span>
                      <ImageUploader
                        single
                        value={row.rankLogo ? [row.rankLogo] : []}
                        onChange={(urls) => {
                          const next = [...data];
                          next[index] = { ...row, rankLogo: urls[0] || '' };
                          formApi?.setValue('data', next);
                        }}
                        max={1}
                      />
                      <Input
                        style={{ width: 220 }}
                        value={row.rankName || ''}
                        placeholder={t('等级名称')}
                        onChange={(next) => {
                          const nextRows = [...data];
                          nextRows[index] = { ...row, rankName: next };
                          formApi?.setValue('data', nextRows);
                        }}
                      />
                      {data.length > 1 ? (
                        <Button
                          theme="borderless"
                          type="danger"
                          size="small"
                          onClick={() => formApi?.setValue('data', data.filter((_, i) => i !== index))}
                        >
                          {t('删除')}
                        </Button>
                      ) : null}
                    </div>
                  ))}
                  {data.length < 10 ? (
                    <Button
                      theme="borderless"
                      type="primary"
                      icon={<IconPlus />}
                      onClick={() => {
                        const next = [...data];
                        next.push({
                          rankLevel: data.length + 1,
                          rankName: LEVEL_NAMES[data.length] ?? `Vip${data.length + 1}`,
                          rankLogo: '',
                        });
                        formApi?.setValue('data', next);
                      }}
                    >
                      {t('添加等级')}
                    </Button>
                  ) : null}
                </div>
              );
            },
          },
        ],
      },
    ],
    toPayload: (values) => {
      const data = (values.data || []).map((row: RankRow, index: number) => ({
        ...row,
        rankLevel: row.rankLevel ?? index + 1,
      }));
      if (data.some((row: RankRow) => !String(row.rankName || '').trim())) {
        throw new Error(t('请填写所有等级名称'));
      }
      return { data };
    },
  };

  const columns = [
    {
      title: t('等级'),
      dataIndex: 'rankLevel',
      width: 120,
      render: (value: number) => <span className="member-level-badge">vip{value}</span>,
    },
    {
      title: t('会员图标'),
      dataIndex: 'rankLogo',
      width: 140,
      render: (value: string) =>
        value ? <Image src={value} height={20} /> : <span>-</span>,
    },
    {
      title: t('会员名称'),
      dataIndex: 'rankName',
      render: (value: string) => <span>{value || '-'}</span>,
    },
  ];

  return (
    <div className="member-levels-page">
      <Card shadows="hover">
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 14 }}>
          <Text strong>{t('会员等级')}</Text>
          <Button theme="solid" type="primary" onClick={openEdit}>
            {t('编辑等级')}
          </Button>
        </div>
        <Table
          loading={loading}
          rowKey="rankLevel"
          dataSource={rows}
          columns={columns as never}
          pagination={false}
          empty={loading ? null : <Text type="tertiary">{t('暂无数据')}</Text>}
        />
      </Card>

      <FormDrawer
        visible={drawerOpen}
        mode={drawerMode}
        initialValues={drawerInitial}
        config={drawerConfig}
        saving={saving}
        onCancel={() => setDrawerOpen(false)}
        onSubmit={async (payload) => {
          setSaving(true);
          try {
            // 后端 update 走 UserRankEditParam：rankType 必填、rankType=1 时 growUpSetting
            // 必填，只发 {data} 必被拒；原 create 分支发 {data:[...]} 给扁平 UserRankDTO
            // 会静默写出一条全空等级。故统一走 update 并带齐 detail 回填的配置
            // （口径与 MemberLevelPros 提交一致），data 行保留原行全部字段防丢失。
            await apiPost('/user/userRank/update', {
              id: 0,
              rankType: rankConfig?.rankType ?? 1,
              userRankConfig: rankConfig?.userRankConfig ?? {
                type: 1,
                rankAfterMonth: 12,
                useMonth: 12,
              },
              growUpSetting: rankConfig?.growUpSetting ?? {
                buyOrder: 0,
                buyOrderNumber: 0,
                buyOrderGrowth: 0,
                evpi: 0,
                evpiGrowth: 0,
                bindPhone: 0,
                bindPhoneGrowth: 0,
              },
              data: (payload.data || []).map((row: RankRow, index: number) => ({
                ...row,
                rankId: (row as any).rankId || undefined,
                rankLevel: Number(row.rankLevel) || index + 1,
                rankName: String(row.rankName || ''),
                rankLogo: row.rankLogo || '',
              })),
            });
            Toast.success({ content: t('操作成功') });
            setDrawerOpen(false);
            reload();
          } catch (error) {
            Toast.error({ content: error instanceof Error ? error.message : t('保存失败') });
          } finally {
            setSaving(false);
          }
        }}
      />
    </div>
  );
}
